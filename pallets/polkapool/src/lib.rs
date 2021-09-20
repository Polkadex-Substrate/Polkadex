// This file is part of Polkadex.

// Copyright (C) 2020-2021 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use codec::Codec;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch,
    traits::{Get, OriginTrait},
};
use frame_support::dispatch::{Dispatchable, DispatchInfo, GetDispatchInfo, PostDispatchInfo};
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;
use frame_support::traits::{Contains, Randomness};
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiLockableCurrency, MultiReservableCurrency};
use polkadex_primitives::assets::AssetId;
use polkadex_primitives::BlockNumber;
use rand::{SeedableRng, seq::SliceRandom};
use rand_chacha::ChaChaRng;
use sp_arithmetic::traits::*;
use sp_arithmetic::traits::{Bounded, One, SaturatedConversion, Saturating, Zero};
use sp_core::H256;
use sp_runtime::traits::{SignedExtension, DispatchInfoOf};
use sp_std::boxed::Box;
use sp_std::collections::vec_deque::VecDeque;
use sp_std::vec;
use sp_std::vec::Vec;

use crate::traits::DynamicStaker;

pub mod traits;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// The aggregated origin which the dispatch will take.
    type Origin: OriginTrait<PalletsOrigin=Self::PalletsOrigin>
    + From<Self::PalletsOrigin>
    + IsType<<Self as frame_system::Config>::Origin>;

    /// The caller origin, overarching type of all pallets origins.
    type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>> + Codec + Clone + Eq;

    /// The aggregated call type.
    type Call: Parameter
    + Dispatchable<Origin=<Self as Config>::Origin>
    + GetDispatchInfo
    + From<frame_system::Call<Self>>;
    /// Balance Type
    type Balance: Parameter
    + Member
    + AtLeast32BitUnsigned
    + Default
    + Copy
    + MaybeSerializeDeserialize
    + Clone
    + Zero
    + One
    + PartialOrd
    + Bounded;
    /// Module that handles tokens
    type Currency: MultiReservableCurrency<Self::AccountId, CurrencyId=AssetId, Balance=Self::Balance>
    + MultiLockableCurrency<Self::AccountId>;
    /// Min amount that must be staked
    type MinStakeAmount: Get<Self::Balance>;
    /// Maximum allowed Feeless Transactions in a block
    type MaxAllowedWeight: Get<Weight>;
    /// Min Stake Period
    type MinStakePeriod: Get<BlockNumber>;
    /// Max number of stakes per account
    type MaxStakes: Get<usize>;
    /// Randomness Source
    type RandomnessSource: Randomness<H256, BlockNumber>;
    /// Call Filter
    type CallFilter: Contains<<Self as Config>::Call>;
    /// DynamicStaking Config
    type DynamicStaking: DynamicStaker<<Self as Config>::Call, Self::Balance>;
    /// Contains function to retrieve
    /// Minimum Stake per Call
    type MinStakePerWeight: Get<u128>;
    /// The Governance Origin that can slash stakes
    type GovernanceOrigin: EnsureOrigin<<Self as Config>::Origin, Success=Self::AccountId>;
}

#[derive(Decode, Encode, Copy, Clone)]
pub struct Stake<T: Config + frame_system::Config> {
    pub staked_amount: T::Balance,
    pub unlocking_block: T::BlockNumber,
}

impl<T: Config + frame_system::Config> Default for Stake<T> {
    fn default() -> Self {
        Stake {
            staked_amount: 0_u128.saturated_into(),
            unlocking_block: 1_u32.saturated_into(),
        }
    }
}

impl<T: Config + frame_system::Config> Stake<T> {
    pub fn new(stake: T::Balance, unlock: T::BlockNumber) -> Stake<T> {
        Stake {
            staked_amount: stake,
            unlocking_block: unlock,
        }
    }
}

#[derive(Decode, Encode, Clone)]
pub struct StakeInfo<T: Config + frame_system::Config> {
    stakes: VecDeque<Stake<T>>,
}

impl<T: Config + frame_system::Config> Default for StakeInfo<T> {
    fn default() -> Self {
        StakeInfo {
            stakes: VecDeque::new(),
        }
    }
}

impl<T: Config + frame_system::Config> StakeInfo<T> {
    pub fn new(stake: T::Balance, unlock: T::BlockNumber) -> StakeInfo<T> {
        let mut queue = VecDeque::new();
        queue.push_back(Stake::new(stake, unlock));
        StakeInfo { stakes: queue }
    }

    pub fn push(&mut self, stake: T::Balance, unlock: T::BlockNumber) -> Result<(), Error<T>> {
        if self.stakes.len() < T::MaxStakes::get() {
            self.stakes.push_back(Stake::new(stake, unlock));
            return Ok(());
        }
        Err(Error::<T>::MaxStakesExceededForAccount)
    }

    pub fn claimable_stakes(&mut self, current_block: T::BlockNumber) -> Vec<Stake<T>> {
        let mut claimable_stakes = vec![];
        while let Some(stake) = self.stakes.pop_front() {
            if stake.unlocking_block <= current_block {
                claimable_stakes.push(stake);
                continue;
            }
            break;
        }
        claimable_stakes
    }

    pub fn total_stake(&mut self) -> T::Balance {
        let mut total: T::Balance = 0u128.saturated_into();
        while let Some(stake) = self.stakes.pop_front() {
            total += stake.staked_amount;
        }
        total
    }
}

#[derive(Decode, Encode, Copy, Clone)]
pub struct Ext<Call, Origin> {
    pub call: Call,
    pub origin: Origin,
}

#[derive(Decode, Encode, Clone)]
pub struct ExtStore<Call, Origin> {
    /// vector of eligible feeless extrinsics
    pub store: Vec<Ext<Call, Origin>>,
    /// Total Weight of the stored extrinsics
    pub total_weight: Weight,
}

impl<Call, Origin> Default for ExtStore<Call, Origin> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            total_weight: 0,
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as Polkapool {
        /// All users and their staked amount
        /// (when they can claim, accountId => Balance)
        pub StakedUsers get(fn staked_users):  map hasher(blake2_128_concat) T::AccountId => StakeInfo<T>;

        /// Feeless Extrinsics stored for next block
        pub TxnsForNextBlock get(fn get_next_block_txns): ExtStore<<T as Config>::Call, <T as Config>::PalletsOrigin>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Balance = <T as Config>::Balance,
        Call = <T as Config>::Call,
        PostInfo = <<T as Config>::Call as Dispatchable>::PostInfo,
    {
        FeelessExtrinsicAccepted(Call),
        FeelessCallFailedToExecute(PostInfo),
        FeelessCallExecutedSuccessfully(PostInfo),
        FeelessExtrinsicsExecuted(Vec<Call>),
        StakeSlashed(AccountId, Balance),
        Unstaked(AccountId, Balance),
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Config>
    {
        StakeAmountTooSmall,
        NotEnoughBalanceToStake,
        UnlockingFailedCurrentBlockNumberLow,
        StakeNotFound,
        FailedToDepositStakedAmount,
        NoMoreFeelessTxnsForThisBlock,
        BadOrigin,
        InvalidCall,
        Overflow,
        BadCall,
        FailedToMoveBalanceToReserve,
        MaxStakesExceededForAccount
    }
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: <T as frame_system::Config>::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_initialize(_n: T::BlockNumber) -> Weight {
            // Load the exts and clear the storage
            let mut stored_exts: ExtStore<<T as Config>::Call, <T as Config>::PalletsOrigin> = <TxnsForNextBlock<T>>::take();
            let base_weight: Weight = T::DbWeight::get().reads_writes(1, 1);
            let mut total_weight: Weight = 0;
            let seed = <T as Config>::RandomnessSource::random_seed();
            let mut rng = ChaChaRng::from_seed(*seed.0.as_fixed_bytes());
            stored_exts.store.shuffle(&mut rng);
            total_weight = total_weight.saturating_add(base_weight);
            // Start executing
            for ext in stored_exts.store{
                total_weight = total_weight.saturating_add(ext.call.get_dispatch_info().weight);
                if total_weight > T::MaxAllowedWeight::get() {
                    total_weight = total_weight.saturating_sub(ext.call.get_dispatch_info().weight);
                    break;
                }
                match ext.call.dispatch(ext.origin.into()) {
                    Ok(post_info) => {
                        Self::deposit_event(RawEvent::FeelessCallExecutedSuccessfully(post_info));
                    }
                    Err(post_info_with_error) => {
                        Self::deposit_event(RawEvent::FeelessCallFailedToExecute(post_info_with_error.post_info));
                    }
                }

            }

            total_weight
        }

        // TODO: Update the weights to include swap transaction's weight
        /// ## Claim Fee-less Transaction
        /// * `stake_amount`: StakePricePerWeight
        /// * `call`: Call from Contracts Pallet
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn claim_feeless_transaction(origin, stake_price: <T as Config>::Balance, call: Box<<T as Config>::Call>) -> dispatch::DispatchResult {
            let who = ensure_signed(origin.clone())?;
            ensure!(origin.clone().into().is_ok(),Error::<T>::BadOrigin);
            ensure!(T::CallFilter::contains(&call), Error::<T>::InvalidCall);

            let call_weight =  call.get_dispatch_info().weight;

            let mut stored_exts: ExtStore<<T as Config>::Call, <T as Config>::PalletsOrigin> = Self::get_next_block_txns();
            stored_exts.total_weight = stored_exts.total_weight.saturating_add(call_weight);
            ensure!(stored_exts.total_weight <= T::MaxAllowedWeight::get(), Error::<T>::NoMoreFeelessTxnsForThisBlock);

            // Calculates the stake amount and the stake period for the given call
            let (stake_amount, stake_period) = Self::calculate_stake_params(stake_price, call_weight);
            let current_block: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

            // Load account Info
            let mut staked_info: StakeInfo<T> = Self::staked_users(who.clone());
            staked_info.push(stake_amount,stake_period+current_block)?;

             //Reserve stake amount
            ensure!(T::Currency::reserve(AssetId::POLKADEX, &who, stake_amount).is_ok(),Error::<T>::FailedToMoveBalanceToReserve);
            // TODO: Replace reserver with T::Currency::set_lock() since that makes more sense

            let origin = <T as Config>::Origin::from(origin);

            // Store the transactions randomize and execute on next block's initialize
            stored_exts.store.push(Ext{
                call: *call.clone(),
                origin: origin.caller().clone()
            });

            <StakedUsers<T>>::insert(who,staked_info);
            <TxnsForNextBlock<T>>::put(stored_exts);
            Self::deposit_event(RawEvent::FeelessExtrinsicAccepted(*call));
            Ok(())
        }

        /// ## Unstake
        /// Returns staked tokens back to origin if `origin` is a staked user
        #[weight = 10000]
        pub fn unstake(origin) -> DispatchResult {
            let who = ensure_signed(origin.clone())?;
            ensure!(origin.into().is_ok(),Error::<T>::BadOrigin);
            ensure!(<StakedUsers<T>>::contains_key(&who),Error::<T>::StakeNotFound);
            let mut stake_info: StakeInfo<T> = <StakedUsers<T>>::get(&who);
            let current_block_no: T::BlockNumber = <frame_system::Pallet<T>>::block_number();
            let mut total: T::Balance = 0u128.saturated_into();
            for stake in stake_info.claimable_stakes(current_block_no){
                T::Currency::unreserve(AssetId::POLKADEX, &who, stake.staked_amount);
                total += stake.staked_amount;
            }
            Self::deposit_event(RawEvent::Unstaked(who,total));
            Ok(())
        }

        /// ## Slash Stake
        /// Slash stake of account by the Governance
        #[weight = 10000]
        pub fn slash_stake(origin, account: T::AccountId) -> DispatchResult {
            let origin = <T as Config>::Origin::from(origin);
            T::GovernanceOrigin::ensure_origin(origin)?;
            let mut stake_info: StakeInfo<T> = <StakedUsers<T>>::take(&account);
            T::Currency::withdraw(AssetId::POLKADEX,&account,stake_info.total_stake())?;
            Self::deposit_event(RawEvent::StakeSlashed(account,stake_info.total_stake()));
            Ok(())
        }
    }
}

impl<T: Config> Module<T> {
    // Calculates the stake amount and staking period
    // The staking period will vary between 28-56 days. The heavier the transaction, longer the period.
    // A transaction consuming the full allocated weight lock the tokens for 56 days.
    pub fn calculate_stake_params(
        stake_price: T::Balance,
        call_weight: Weight,
    ) -> (T::Balance, T::BlockNumber) {
        // Calculate the min requirements
        let stake_amount = stake_price.saturating_mul(call_weight.saturated_into());
        let mut stake_period: T::BlockNumber = T::MinStakePeriod::get().saturated_into(); // 28 days
        // Add a extra staking period based on the fraction of total weight allocation this call occupies
        if let Some(allocation_inverse_fraction) =
        T::MaxAllowedWeight::get().checked_div(call_weight)
        {
            if let Some(extra_stake_period) =
            stake_period.checked_div(&allocation_inverse_fraction.saturated_into())
            {
                stake_period = stake_period.saturating_add(extra_stake_period);
            }
        }
        (stake_amount, stake_period)
    }
}

#[derive(Encode, Decode, Clone, Eq, PartialEq)]
pub struct DynamicStaking<T: Config>(#[codec(compact)] T::Balance);

impl<T: Config> DynamicStaking<T> where <T as Config>::Call: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo> {
    /// NOTE: The function below is modified from paritytech's pallet transaction payment
    /// Get an appropriate priority for a transaction with the given length and info.
    ///
    /// This will try and optimise the `stake/weight` `stake/length`, whichever is consuming more of the
    /// maximum corresponding limit.
    ///
    /// For example, if a transaction consumed 1/4th of the block length and half of the weight, its
    /// final priority is `stake * min(2, 4) = fee * 2`. If it consumed `1/4th` of the block length
    /// and the entire block weight `(1/1)`, its priority is `stake * min(1, 4) = fee * 1`. This means
    ///  that the transaction which consumes more resources (either length or weight) with the same
    /// `fee` ends up having lower priority.
    fn calculate_priority(len: usize, info: &DispatchInfoOf<<T as Config>::Call>, stake: T::Balance) -> TransactionPriority {
        let weight_saturation = T::BlockWeights::get().max_block / info.weight.max(1);
        let max_block_length = *T::BlockLength::get().max.get(DispatchClass::Normal);
        let len_saturation = max_block_length as u64 / (len as u64).max(1);
        let coefficient: T::Balance =
            weight_saturation.min(len_saturation).saturated_into::<T::Balance>();
        stake.saturating_mul(coefficient).saturated_into::<TransactionPriority>()
    }
}

impl<T: Config> sp_std::fmt::Debug for DynamicStaking<T> {
    #[cfg(feature = "std")]
    fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        write!(f, "DynamicStaking<{:?}>", self.0)
    }
    #[cfg(not(feature = "std"))]
    fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        Ok(())
    }
}

impl<T: Config> SignedExtension for DynamicStaking<T> where <T as Config>::Call: Dispatchable<Info=DispatchInfo, PostInfo=PostDispatchInfo> {
    const IDENTIFIER: &'static str = "DynamicStaking";
    type AccountId = T::AccountId;
    type Call = <T as Config>::Call;
    type AdditionalSigned = ();
    type Pre = ();

    fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
        Ok(())
    }

    fn validate(&self, _who: &Self::AccountId, call: &<T as Config>::Call, info: &DispatchInfoOf<<T as Config>::Call>, len: usize) -> TransactionValidity {
        if T::DynamicStaking::filter(call) {
            let stake: T::Balance = T::DynamicStaking::get_stake(call);
            Ok(ValidTransaction { priority: Self::calculate_priority(len, info, stake), ..Default::default() })
        } else {
            Ok(ValidTransaction::default())
        }
    }
}
