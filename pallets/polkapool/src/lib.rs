#![cfg_attr(not(feature = "std"), no_std)]

use codec::Codec;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, traits::{Get, OriginTrait},
};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo};
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;
use frame_support::traits::{Filter, Randomness};
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use polkadex_primitives::assets::AssetId;
use polkadex_primitives::BlockNumber;
use sp_arithmetic::traits::{Bounded, One, SaturatedConversion, Saturating, Zero};
use sp_core::H256;
use sp_consensus_vrf::schnorrkel;
use sp_std::vec::Vec;
use sp_std::boxed::Box;
use frame_system::{
    limits::{BlockWeights}
};
use rand_chacha::ChaChaRng;
use rand::{seq::SliceRandom, SeedableRng};
/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// The aggregated origin which the dispatch will take.
    type Origin: OriginTrait<PalletsOrigin = Self::PalletsOrigin>
    + From<Self::PalletsOrigin>
    + IsType<<Self as frame_system::Config>::Origin>;

    /// The caller origin, overarching type of all pallets origins.
    type PalletsOrigin: From<frame_system::RawOrigin<Self::AccountId>> + Codec + Clone + Eq;

    /// The aggregated call type.
    type Call: Parameter
    + Dispatchable<Origin = <Self as Config>::Origin>
    + GetDispatchInfo
    + From<frame_system::Call<Self>>;
    /// Balance Type
    type Balance: Parameter
    + Member
    + AtLeast32BitUnsigned
    + Default
    + Copy
    + MaybeSerializeDeserialize + Clone + Zero + One + PartialOrd + Bounded;
    /// Module that handles tokens
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId=AssetId,
        Balance=Self::Balance,
    >;
    /// Min amount that must be staked
    type MinStakeAmount: Get<Self::Balance>;
    /// Maximum allowed Feeless Transactions in a block
    type MaxAllowedWeight: Get<Weight>;
    /// Min Stake Period
    type MinStakePeriod: Get<Self::BlockNumber>;
    /// Randomness Source
    type RandomnessSource: Randomness<H256, BlockNumber>;
    /// Call Filter
    type CallFilter: Filter<<Self as Config>::Call>;
    //Minimum Stake per Call
    type MinStakePerWeight : Get<u128>;

    type GovernanceOrigin: EnsureOrigin<<Self as Config>::Origin, Success = Self::AccountId>;

}



#[derive(Decode, Encode, Copy, Clone)]
pub struct StakeInfo<T: Config + frame_system::Config> {
    pub staked_amount: T::Balance,
    pub unlocking_block: T::BlockNumber,
}

impl<T: Config + frame_system::Config> Default for StakeInfo<T> {
    fn default() -> Self {
        StakeInfo { staked_amount: 0_u128.saturated_into(), unlocking_block: 1_u32.saturated_into() }
    }
}

impl<T: Config + frame_system::Config> StakeInfo<T> {
    pub fn new(stake: T::Balance, unlock: T::BlockNumber) -> StakeInfo<T> {
        StakeInfo {
            staked_amount: stake,
            unlocking_block: unlock,
        }
    }
}

#[derive(Decode, Encode, Copy, Clone)]
pub struct MovingAverage<T: Config> {
    pub amount: <T as Config>::Balance,
    pub count: <T as Config>::Balance,
}

impl<T: Config> Default for MovingAverage<T> {
    fn default() -> Self {
        MovingAverage { amount: 0_u128.saturated_into(), count: 0_u128.saturated_into() }
    }
}

impl<T: Config> MovingAverage<T> {
    pub fn update_stake_amount(&mut self, stake_amount: <T as Config>::Balance) {
        let new_count = self.count.saturating_add(1u128.saturated_into());
        self.amount = self.amount.saturating_mul(self.count.clone()).saturating_add(stake_amount) / new_count;
        self.count = new_count
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

impl<Call,Origin> Default for ExtStore<Call, Origin> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            total_weight: 0
        }
    }
}


decl_storage! {
    trait Store for Module<T: Config> as Polkapool {
        /// All users and their staked amount
        /// (when they can claim, accountId => Balance)
        pub StakedUsers get(fn staked_users):  map hasher(blake2_128_concat) T::AccountId => StakeInfo<T>;

        /// StakeMovingAverage
        /// TODO: Set StakeMovingAverage as MinStakeAmount if it is zero
        pub StakeMovingAverage get(fn get_stake_moving_average): MovingAverage<T>;

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
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Config>
    {
        StakeAmountTooSmall,
        NotEnoughBalanceToStake,
        NoMoreFeelessTxnsForThisBlock,
        BadOrigin,
        InvalidCall,
        Overflow,
        BadCall
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
            // Start executing
            for ext in stored_exts.store{
                total_weight = total_weight + ext.call.get_dispatch_info().weight;
                match ext.call.dispatch(ext.origin.into()) {
                    Ok(post_info) => {
                        Self::deposit_event(RawEvent::FeelessCallExecutedSuccessfully(post_info));
                    }
                    Err(post_info_with_error) => {
                        Self::deposit_event(RawEvent::FeelessCallFailedToExecute(post_info_with_error.post_info));
                    }
                }

            }
            total_weight = total_weight + base_weight;
            total_weight
        }

        // TODO: Update the weights to include swap transaction's weight
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn claim_feeless_transaction(origin, stake_amount: <T as Config>::Balance, call: Box<<T as Config>::Call>) -> dispatch::DispatchResult {
            let who = ensure_signed(origin.clone())?;
            ensure!(origin.clone().into().is_ok(),Error::<T>::BadOrigin);
            ensure!(T::CallFilter::filter(&call), Error::<T>::InvalidCall);
            let origin = <T as Config>::Origin::from(origin);

            let minimum_stake_amount = T::MinStakePerWeight::get().checked_mul(call.get_dispatch_info().weight as u128).ok_or(<Error<T>>::Overflow)?;
            ensure!(stake_amount >= minimum_stake_amount.saturated_into(), Error::<T>::StakeAmountTooSmall); // TODO
            ensure!(stake_amount <= T::Currency::free_balance(AssetId::POLKADEX,&who), Error::<T>::NotEnoughBalanceToStake);

            let mut stored_exts: ExtStore<<T as Config>::Call, <T as Config>::PalletsOrigin> = Self::get_next_block_txns();
            ensure!(stored_exts.total_weight < T::MaxAllowedWeight::get(), Error::<T>::NoMoreFeelessTxnsForThisBlock);

            // Update the moving average of stake amount
            let mut stake_moving_average: MovingAverage<T> = Self::get_stake_moving_average();
            stake_moving_average.update_stake_amount(stake_amount.clone());

            // Get current block number
            let current_block_number: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

            // Store the staking record
            let mut staked_amount = Self::staked_users(who.clone());
            staked_amount.staked_amount += stake_amount;
            staked_amount.unlocking_block = current_block_number + T::MinStakePeriod::get();


            // Store the transactions randomize and execute on next block's initialize
            stored_exts.store.push(Ext{
                call: *call.clone(),
                origin: origin.caller().clone()
            });
            stored_exts.total_weight += call.get_dispatch_info().weight;

            <StakedUsers<T>>::insert(who.clone(),staked_amount);
            <TxnsForNextBlock<T>>::put(stored_exts);
            <StakeMovingAverage<T>>::put(stake_moving_average);
            Self::deposit_event(RawEvent::FeelessExtrinsicAccepted(*call));
            Ok(())
        }

        #[weight = 10000]
        pub fn slash_stake(origin, account: T::AccountId) -> DispatchResult {
            let origin = <T as Config>::Origin::from(origin);
            T::GovernanceOrigin::ensure_origin(origin)?;
            let stake = <StakedUsers<T>>::get(&account);
            T::Currency::slash(AssetId::POLKADEX,&account,stake.staked_amount);
            Self::deposit_event(RawEvent::StakeSlashed(account,stake.staked_amount));
            Ok(())
        }
    }
}

