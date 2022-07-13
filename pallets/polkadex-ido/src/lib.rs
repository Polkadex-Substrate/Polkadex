// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
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
// Clippy warning diabled for to many arguments on line#157
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unused_unit)]

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        tokens::fungibles::{Create, Inspect, Mutate, Transfer, Unbalanced},
        EnsureOrigin, Get, Randomness, WithdrawReasons,
    },
    PalletId,
};
use frame_support::traits::tokens::fungible;
use frame_system as system;
use frame_system::ensure_signed;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use scale_info::StaticTypeInfo;
use sp_core::H256;
use sp_runtime::{
    traits::{AccountIdConversion, Saturating, Zero},
    Perbill, Perquintill, SaturatedConversion,
};
use sp_std::{
    cmp::{max, min},
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    prelude::*,
};

pub use pallet::*;


use pallet_polkadex_ido_primitives::{FundingRoundWithPrimitives, VoteStat, StringAssetId};


use polkadex_primitives::assets::AssetId;
use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement};

type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::tokens::fungibles::{Create, Inspect, Mutate},
        PalletId,
    };
    use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
    use sp_core::{H160, H256};
    use sp_runtime::traits::One;
    use sp_std::prelude::*;

    use super::*;


    use polkadex_primitives::assets::AssetId;
    use pallet_polkadex_ido_primitives::AccountId;

    /// The module configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The origin which may attests the investor to take part in the IDO pallet.
        type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
        /// The treasury mechanism.
        #[pallet::constant]
        type TreasuryAccountId: Get<Self::AccountId>;
        /// Balances Pallet
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId> + fungible::Inspect<Self::AccountId>;
        /// The basic amount of funds that must be reserved for an Polkadex.
        #[pallet::constant]
        type IDOPDXAmount: Get<BalanceOf<Self>>;
        /// Maximum supply for IDO
        #[pallet::constant]
        type MaxSupply: Get<BalanceOf<Self>>;
        /// The generator used to supply randomness to IDO
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
        /// Randomness Source for random participant seed
        type RandomnessSource: Randomness<H256, Self::BlockNumber>;
        /// The IDO's module id
        #[pallet::constant]
        type ModuleId: Get<PalletId>;
        /// Default voting period
        #[pallet::constant]
        type DefaultVotingPeriod: Get<Self::BlockNumber>;
        /// Default investor locking period
        #[pallet::constant]
        type DefaultInvestorLockPeriod: Get<Self::BlockNumber>;
        /// Minimum deposit to create PDEX account for round id
        #[pallet::constant]
        type ExistentialDeposit: Get<BalanceOf<Self>>;

        /// One PDEX amount in u128
        #[pallet::constant]
        type OnePDEX : Get<u128>;

        type AssetManager: Create<<Self as frame_system::Config>::AccountId>
        + Mutate<<Self as frame_system::Config>::AccountId, Balance=u128, AssetId=u128>
        + Inspect<<Self as frame_system::Config>::AccountId>
        + Transfer<<Self as frame_system::Config>::AccountId>
        + Unbalanced<<Self as frame_system::Config>::AccountId>;
    }
    /// All information for funding round
    #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct FundingRound<T: Config> {
        pub token_a: AssetId,
        pub creator: T::AccountId,
        pub amount: BalanceOf<T>,
        pub token_b: AssetId,
        pub project_info_cid: Vec<u8>,
        pub vesting_end_block: T::BlockNumber,
        pub vesting_per_block: BalanceOf<T>,
        pub start_block: T::BlockNumber,
        pub min_allocation: BalanceOf<T>,
        pub max_allocation: BalanceOf<T>,
        pub token_a_priceper_token_b: BalanceOf<T>,
        pub close_round_block: T::BlockNumber,
        pub actual_raise: BalanceOf<T>,
    }

    impl<T: Config> FundingRound<T> {
        fn from(
            cid: Vec<u8>,
            token_a: AssetId,
            creator: T::AccountId,
            amount: BalanceOf<T>,
            token_b: AssetId,
            vesting_end_block: T::BlockNumber,
            vesting_per_block: BalanceOf<T>,
            start_block: T::BlockNumber,
            min_allocation: BalanceOf<T>,
            max_allocation: BalanceOf<T>,
            token_a_priceper_token_b: BalanceOf<T>,
            close_round_block: T::BlockNumber,
        ) -> Self {
            FundingRound {
                token_a,
                creator,
                amount,
                token_b,
                project_info_cid: cid,
                vesting_end_block,
                vesting_per_block,
                start_block,
                min_allocation,
                max_allocation,
                token_a_priceper_token_b,
                close_round_block,
                actual_raise: Zero::zero(),
            }
        }

        pub fn to_primitive(&self) -> FundingRoundWithPrimitives<T::AccountId> {
            FundingRoundWithPrimitives {
                token_a: StringAssetId::from(self.token_a),
                creator: self.creator.clone(),
                amount: self.amount.saturated_into(),
                token_b: StringAssetId::from(self.token_b),
                vesting_per_block: self.vesting_per_block.saturated_into(),
                start_block: self.start_block.saturated_into(),
                vesting_end_block: self.vesting_end_block.saturated_into(),
                project_info_cid: self.project_info_cid.clone(),
                min_allocation: self.min_allocation.saturated_into(),
                max_allocation: self.max_allocation.saturated_into(),
                token_a_priceper_token_b: self.token_a_priceper_token_b.saturated_into(),
                close_round_block: self.close_round_block.saturated_into(),
                actual_raise: self.actual_raise.saturated_into(),
            }
        }

        pub fn token_a_price_per_1e12_token_b(&self) -> Perbill {
            let token_a_priceper_token_b: u128 = self.token_a_priceper_token_b.saturated_into();
            Perbill::from_rational(token_a_priceper_token_b, T::OnePDEX::get())
        }

        pub fn token_a_price_per_1e12_token_b_balance(&self) -> BalanceOf<T> {
            let token_a_priceper_token_b: u128 = self.token_a_priceper_token_b.saturated_into();
            let p = (token_a_priceper_token_b as f64 / T::OnePDEX::get() as f64) as u128;
            p.saturated_into()
        }
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::call]
    impl<T: Config> Pallet<T> {

        /// Registers a funding round with the amount as the total allocation for this round and vesting period.
        ///
        /// # Parameters
        /// * `cid` : IPFS cid
        /// * `token_a`: The Project token
        /// * `amount`: Amount for funding round
        /// * `token_b`: Token in which funding is received
        /// * `vesting_per_block`: Vesting per block
        /// * `funding_period`: Number of blocks from the current block for funding/show interest in funding round
        /// * `min_allocation`: Minimum allocation of funds investor can invest
        /// * `max_allocation`: Maximum allocation of funds investor can invest
        /// * `token_a_priceper_token_b`: Priceper amount for project token
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn register_round(
            origin: OriginFor<T>,
            cid: Vec<u8>,
            token_a: AssetId,
            amount: BalanceOf<T>,
            token_b: AssetId,
            vesting_per_block: BalanceOf<T>,
            funding_period: T::BlockNumber,
            min_allocation: BalanceOf<T>,
            max_allocation: BalanceOf<T>,
            token_a_priceper_token_b: BalanceOf<T>,
        ) -> DispatchResult {
            let team: T::AccountId = ensure_signed(origin)?;
            //TODO check if funder have the token_a available and reserve them.
            // CID len must be less than or equal to 100
            ensure!(cid.len() <= 100, <Error<T>>::CidReachedMaxSize);
            ensure!(min_allocation <= max_allocation, <Error<T>>::MinAllocationMustBeEqualOrLessThanMaxAllocation);
            ensure!(vesting_per_block > Zero::zero(), <Error<T>>::VestingPerBlockMustGreaterThanZero);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let start_block = current_block_no.clone().saturating_add(1_u128.saturated_into());
            let close_round_block = current_block_no.saturating_add(funding_period);
            let token_a_priceper_token_b_perquintill = Perbill::from_rational(token_a_priceper_token_b, 1_000_000_000_000_u128.saturated_into());
            ensure!(!token_a_priceper_token_b_perquintill.is_zero(), <Error<T>>::PricePerTokenCantBeZero);
            ensure!(start_block < close_round_block, <Error<T>>::StartBlockMustBeLessThanEndblock);
            let vesting_period: u32 = (amount / vesting_per_block).saturated_into();
            let vesting_period: T::BlockNumber = vesting_period.saturated_into();
            let vesting_end_block: T::BlockNumber = vesting_period.saturating_add(close_round_block);
            let funding_round: FundingRound<T> = FundingRound::from(
                cid,
                token_a,
                team.clone(),
                amount,
                token_b,
                vesting_end_block,
                vesting_per_block,
                start_block,
                min_allocation,
                max_allocation,
                token_a_priceper_token_b,
                close_round_block,
            ); 
            let (round_id, _) = T::Randomness::random(&(Self::pallet_account_id(), current_block_no, team.clone(), Self::incr_nonce()).encode());
            let round_account_id = Self::round_account_id(round_id.clone());
            //Charge minimum 1 PDEX required to create an account for the round account id
            T::Currency::transfer(&team, &round_account_id, T::ExistentialDeposit::get(), ExistenceRequirement::KeepAlive)?;
            Self::transfer(token_a, &team, &round_account_id, amount.saturated_into())?;
            <InfoFundingRound<T>>::insert(round_id, funding_round);
            <InfoProjectTeam<T>>::insert(team, round_id);
            Self::deposit_event(Event::FundingRoundRegistered(round_id));
            Ok(())
        }

        /// Invest in a funding round
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `amount`: BalanceOf<T>
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn invest(origin: OriginFor<T>, round_id: T::Hash, amount: BalanceOf<T>) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let mut funding_round = <InfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(Self::can_withdraw(funding_round.token_b,&investor_address, amount.saturated_into()).is_ok(), Error::<T>::BalanceInsufficientForInteresetedAmount);
            let amount_in_token_a = if T::OnePDEX::get().saturated_into::<BalanceOf<T>>() >= funding_round.token_a_priceper_token_b {
                funding_round.token_a_price_per_1e12_token_b().saturating_reciprocal_mul(amount) 
            } else {
                amount / funding_round.token_a_price_per_1e12_token_b_balance()
            };
            ensure!(amount_in_token_a <= funding_round.max_allocation && amount_in_token_a >= funding_round.min_allocation, Error::<T>::NotAValidAmount);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block_no >= funding_round.start_block && current_block_no < funding_round.close_round_block, <Error<T>>::NotAllowed);
            let total_raise = funding_round.actual_raise.saturating_add(amount_in_token_a);
            let round_account_id = Self::round_account_id(round_id.clone());

            // First come first serve basis 
            if total_raise >= funding_round.amount{
                return Err(<Error<T>>::NotAllowed.into());
            }
            match Self::transfer(funding_round.token_b, &investor_address, &round_account_id, amount.saturated_into()) {
                Ok(_) => {
                    funding_round.actual_raise = total_raise;
                    Self::deposit_event(Event::ParticipatedInRound(round_id, investor_address.clone()));
                    <InfoFundingRound<T>>::insert(round_id, funding_round);
                    <InvestorShare<T>>::insert(round_id.clone(), investor_address.clone(), amount_in_token_a);
                    <InvestorInvestment<T>>::insert(round_id.clone(), investor_address, amount);
                }
                Err(error) => {
                    Self::deposit_event(Event::ParticipatedInRoundFailed(round_id, investor_address, error));
                }
            }
            Ok(())
        }

        /// Investor claiming for a particular funding round.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn claim_tokens(origin: OriginFor<T>, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            let total_raise: u128 = funding_round.actual_raise.saturated_into();
            let amount: u128 = funding_round.amount.saturated_into();
            ensure!(total_raise >= amount/2, Error::<T>::CannotClaimTokenForFailedIdo);
            let round_account_id = Self::round_account_id(round_id.clone());
            let investor_share = Self::get_investor_share_info(round_id.clone(), investor_address.clone());
            Self::transfer(funding_round.token_a, &round_account_id, &investor_address, investor_share.saturated_into())?;
            Self::deposit_event(Event::TokenClaimed(round_id, investor_address));
            Ok(())
        }

        /// Transfers the raised amount to another address,
        /// only the round creator can call this or the governance.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `beneficiary`: Account Id of Beneficiary
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn claim_raise(origin: OriginFor<T>, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(<InfoProjectTeam<T>>::get(investor_address.clone()) == Some(round_id), Error::<T>::NotAllowed);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            let total_raise: u128 = funding_round.actual_raise.saturated_into();
            let amount: u128 = funding_round.amount.saturated_into();
            ensure!(!<RaiseClaimed<T>>::get(round_id), Error::<T>::RaiseWithdrawnAlready);
            ensure!(total_raise >= amount/2, Error::<T>::CannotWithdrawRaiseForFailedIdo);
            let round_account_id = Self::round_account_id(round_id.clone());
            let mut total_raise: BalanceOf<T> = 0_u128.saturated_into();
            for (_ , _ , amount) in <InvestorInvestment<T>>::iter(){
                total_raise = total_raise.saturating_add(amount);
            }
            Self::transfer(funding_round.token_b, &round_account_id, &investor_address, total_raise.saturated_into())?;
            Self::deposit_event(Event::RaiseClaimed(round_id, investor_address, total_raise.saturated_into()));
            <RaiseClaimed<T>>::insert(round_id.clone(), true);
            Ok(())
        }

        /// Only IDO creator can call this
        /// Transfers the remaining tokens to another address,
        /// only the round creator can call this or the governance.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn withdraw_tokens(origin: OriginFor<T>, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(<InfoProjectTeam<T>>::get(investor_address.clone()) == Some(round_id), Error::<T>::NotAllowed);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            let round_account_id = Self::round_account_id(round_id.clone());
            let rem_tokens = funding_round.amount.saturating_sub(funding_round.actual_raise);
            Self::transfer(funding_round.token_a, &round_account_id, &investor_address, rem_tokens.saturated_into())?;
            Ok(())
        }

        /// Transfers the investment made by the investor
        /// when there's a failed IDO
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn withdraw_investment(origin: OriginFor<T>, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            let total_raise: u128 = funding_round.actual_raise.saturated_into();
            let amount: u128 = funding_round.amount.saturated_into();
            ensure!(total_raise <= amount/2, Error::<T>::CannotWithdrawForSuccesfulIDO);
            let round_account_id = Self::round_account_id(round_id.clone());
            let investment = <InvestorInvestment<T>>::get(&round_id, &investor_address);
            Self::transfer(funding_round.token_b, &round_account_id, &investor_address, investment)?;
            let empty_raise: BalanceOf<T> = 0_u128.saturated_into();
            <InvestorInvestment<T>>::insert(round_id.clone(), investor_address.clone(), empty_raise);
            Self::deposit_event(Event::InvestmentWithdrawn(round_id, investor_address, investment));
            Ok(())
        }
    }

    /// Stores nonce used to create unique ido round id
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub(super) type Nonce<T: Config> = StorageValue<_, u128, ValueQuery>;

    /// Stores funding round info
    #[pallet::storage]
    #[pallet::getter(fn get_funding_round)]
    pub(super) type InfoFundingRound<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        FundingRound<T>,
        OptionQuery,
    >;
    /// Stores project team/ ido creator Info
    #[pallet::storage]
    #[pallet::getter(fn get_team)]
    pub(super) type InfoProjectTeam<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        T::Hash,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_investor_share_info)]
    pub(super) type InvestorShare<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn get_investor_investment_info)]
    pub(super) type InvestorInvestment<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn raise_claimed)]
    pub(super) type RaiseClaimed<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        bool,
        ValueQuery,
    >;
    


    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Funding round has been registered
        FundingRoundRegistered(T::Hash),
        ParticipatedInRound(T::Hash, T::AccountId),
        ParticipatedInRoundFailed(T::Hash, T::AccountId, sp_runtime::DispatchError),
        RaiseClaimed(T::Hash, T::AccountId, BalanceOf<T>),
        TokenClaimed(T::Hash, T::AccountId),
        InvestmentWithdrawn(T::Hash, T::AccountId, BalanceOf<T>)
    }


    #[pallet::error]
    pub enum Error<T> {
        /// Funding round does not exist
        FundingRoundDoesNotExist,
        InsufficientBalance,
        CidReachedMaxSize,
        PricePerTokenCantBeZero,
        MinAllocationMustBeEqualOrLessThanMaxAllocation,
        StartBlockMustBeLessThanEndblock,
        StartBlockMustBeGreaterThanVotingPeriod,
        VestingPerBlockMustGreaterThanZero,
        FundingRoundNotApproved,
        BalanceInsufficientForInteresetedAmount,
        NotAValidAmount,
        NotAllowed,
        WithdrawalBlocked,
        TokenClaimedAlready,
        CannotWithdrawRaiseForFailedIdo,
        CannotClaimTokenForFailedIdo,
        CannotWithdrawForSuccesfulIDO,
        RaiseWithdrawnAlready,
    }
}


impl<T: Config> Pallet<T> {

    /// converts block to balance
    /// # Parameters
    /// * `input` : Block
    fn block_to_balance(input: T::BlockNumber) -> BalanceOf<T> {
        BalanceOf::<T>::from(input.saturated_into::<u32>())
    }

    /// Creates an accound id from round id
    /// # Parameters
    /// * hash : Round id
    pub fn round_account_id(hash: T::Hash) -> T::AccountId {
        T::ModuleId::get().into_sub_account(hash)
    }

    /// Increments and return a nonce
    fn incr_nonce() -> u128 {
        let current_nonce: u128 = <Nonce<T>>::get();
        let (nonce, _) = current_nonce.overflowing_add(1);
        <Nonce<T>>::put(nonce);
        <Nonce<T>>::get()
    }

    /// module wallet account
    pub fn pallet_account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// Returns rounds an investor has invested in
    /// >  Used in RPC call
    /// # Paramteres
    /// * `account` : Account id
    /* pub fn rounds_by_investor(
        account: T::AccountId,
    ) -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        <InvestorShareInfo<T>>::iter()
            .filter_map(|(round_id, investor, _)| {
                if investor != account {
                    None
                } else {
                    if let Some(round_info) = <WhitelistInfoFundingRound<T>>::get(&round_id) {
                        Some((round_id, round_info.to_primitive()))
                    } else {
                        None
                    }
                }
            })
            .collect()
    }*/

    /// Returns rounds created by an account
    /// >  Used in RPC call
    /// # Paramteres
    /// * `account` : Account id
    /* pub fn rounds_by_creator(
        account: T::AccountId,
    ) -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        let whitelisted_funding_round: Vec<_> = <WhitelistInfoFundingRound<T>>::iter()
            .filter_map(|(round_id, round_info)| {
                if round_info.creator != account {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let pending_funding_round: Vec<_> = <InfoFundingRound<T>>::iter()
            .filter_map(|(round_id, round_info)| {
                if round_info.creator != account {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let mut mixed_funding_rounds = Vec::new();
        mixed_funding_rounds.extend_from_slice(&whitelisted_funding_round);
        mixed_funding_rounds.extend_from_slice(&pending_funding_round);
        mixed_funding_rounds
    } */

    /// Returns rounds that are not closed
    /// >  Used in RPC call
    /* pub fn active_rounds() -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        let current_block_no = <frame_system::Pallet<T>>::block_number();
        let mut active_rounds: Vec<_> = <WhitelistInfoFundingRound<T>>::iter()
            .filter_map(|(round_id, round_info)| {
                if round_info.close_round_block < current_block_no {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let pending_funding_round: Vec<_> = <InfoFundingRound<T>>::iter()
            .map(|(round_id, round_info)| (round_id, round_info.to_primitive()))
            .collect();
        active_rounds.extend_from_slice(&pending_funding_round);
        active_rounds
    }*/
    /// Returns Votes statistics for a round
    /// >  Used in RPC call
    /// # Paramteres
    /// * `round_id` : Account id
    /* pub fn votes_stat(round_id: T::Hash) -> VoteStat {
        match <RoundVotes<T>>::try_get(&round_id) {
            Ok(voting) => {
                let yes: BalanceOf<T> = voting
                    .ayes
                    .iter()
                    .map(|a| a.votes)
                    .fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                let no: BalanceOf<T> = voting
                    .nays
                    .iter()
                    .map(|a| a.votes)
                    .fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));

                VoteStat { yes: yes.saturated_into(), no: no.saturated_into() }
            }
            Err(_) => VoteStat { yes: 0, no: 0 },
        }
    } */

    /// Helper function to transfer tokens
    pub fn transfer(token: AssetId, from: &T::AccountId, to: &T::AccountId, amount: BalanceOf<T>) -> Result<(), sp_runtime::DispatchError> {
        match token {
            AssetId::polkadex => {
                T::Currency::transfer(from, to, amount, ExistenceRequirement::KeepAlive)
            }
            AssetId::asset(token_id) => {
                T::AssetManager::transfer(token_id, &from, &to, amount.saturated_into(), false).map(|_| ())
            }
        }
    }

    /// Helper function to check if investor can withdraw an amount
    pub fn can_withdraw(token: AssetId, from_account: &T::AccountId, amount: BalanceOf<T>) -> Result<(), sp_runtime::DispatchError> {
        match token {
            AssetId::polkadex => {
                let account_free_balance: u128 = T::Currency::free_balance(from_account)
                    .saturated_into();
                let new_balance = account_free_balance.checked_sub(amount.saturated_into())
                    .ok_or(Error::<T>::InsufficientBalance)?;
                T::Currency::ensure_can_withdraw(from_account, amount, WithdrawReasons::TRANSFER, new_balance.saturated_into())
            }
            AssetId::asset(token_id) => {
                T::AssetManager::can_withdraw(token_id.into(), from_account, amount.saturated_into()).into_result().map(|_| ())
            }
        }
    }

    /// Takes a list of assets and Returns the asset balance(free balance) belonging to account_id
    pub fn account_balances(assets: Vec<u128>, account_id: T::AccountId) -> Vec<u128> {
        assets.iter().map(|asset| {
            <T as Config>::AssetManager::balance(*asset, &account_id).saturated_into()
        }).collect()
    }
}
