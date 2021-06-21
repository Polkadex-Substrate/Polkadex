#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch, traits::Get,
};
use frame_support::dispatch::{Dispatchable, GetDispatchInfo, PostDispatchInfo, UnfilteredDispatchable};
use frame_support::pallet_prelude::*;
use frame_support::sp_runtime::traits::AtLeast32BitUnsigned;
use frame_support::traits::{IsSubType, Randomness};
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use polkadex_primitives::assets::AssetId;
use polkadex_primitives::BlockNumber;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::H256;

/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// Balance Type
    type Balance: Parameter
    + Member
    + AtLeast32BitUnsigned
    + Default
    + Copy
    + MaybeSerializeDeserialize;
    /// Module that handles tokens
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId=AssetId,
        Balance=Self::Balance,
    >;
    /// Min amount that must be staked
    type MinStakeAmount: Get<Self::Balance>;
    /// Maximum allowed Feeless Transactions in a block, (TODO: Bound the number of transactions based on total weight)
    type MaxAllowedTxns: Get<u128>;
    /// Min Stake Period
    type MinStakePeriod: Get<Self::BlockNumber>;
    /// The overarching call type.
    type Call: Parameter + Dispatchable<Origin=Self::Origin, PostInfo=PostDispatchInfo>
    + GetDispatchInfo + From<frame_system::Call<Self>>
    + UnfilteredDispatchable<Origin=Self::Origin>
    + IsSubType<Call<Self>>
    + IsType<<Self as frame_system::Config>::Call>;
    /// Randomness Source
    type RandomnessSource: Randomness<H256, BlockNumber>;
}

#[derive(Decode, Encode, Default)]
pub struct StakeInfo<T: Config> {
    pub staked_amount: T::Balance,
    pub unlocking_block: T::BlockNumber,
}

impl<T: Config> StakeInfo<T> {
    pub fn new(stake: T::Balance, unlock: T::BlockNumber) -> StakeInfo<T> {
        StakeInfo {
            staked_amount: stake,
            unlocking_block: unlock,
        }
    }
}

#[derive(Decode, Encode, Default)]
pub struct MovingAverage<T: Config> {
    pub amount: T::Balance,
    pub count: T::Balance,
}

impl<T: Config> MovingAverage<T> {
    pub fn update_stake_amount(&mut self, stake_amount: &T::Balance) {
        self.amount = ((self.amount * self.count) + *stake_amount) / (self.count + 1u128.saturated_into());
        self.count += 1u128.saturated_into();
    }
}

#[derive(Decode, Encode, Default)]
pub struct Ext<T: Config> {
    pub call: <T as Config>::Call,
    pub origin: T::Origin,
}

#[derive(Decode, Encode, Default)]
pub struct ExtStore<T: Config> {
    /// vector of eligible feeless extrinsics
    pub store: Vec<Ext<T>>,
    /// Total Weight of the stored extrinsics
    pub total_weight: Weight,
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
        pub TxnsForNextBlock get(fn get_next_block_txns): ExtStore<T>;
    }
}

decl_event!(
    pub enum Event<T>
    where
        // AccountId = <T as frame_system::Config>::AccountId,
        Call = <T as Config>::Call,
    {
        FeelessExtrinsicAccepted(Call),
        FeelessExtrinsicsExecuted(Vec<Call>),
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Config> {
        StakeAmountTooSmall,
        NotEnoughBalanceToStake,
        NoMoreFeelessTxnsForThisBlock
    }
}

// Dispatchable functions allows users to interact with the pallet and invoke state changes.
// These functions materialize as "extrinsics", which are often compared to transactions.
// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        // Errors must be initialized if they are used by the pallet.
        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_initialize(n: T::BlockNumber) -> Weight {
            // Load the exts and clear the storage
            let stored_exts: ExtStore<T> = <TxnsForNextBlock<T>>::take();
            // TODO: Randomize the vector using babe randomness
            let mut used_weight: Weight = Weight::from(0);
            // Start executing
            for ext in stored_exts.store{
                used_weight = used_weight + ext.call.dispatch(ext.origin);
            }
            used_weight
        }

        // TODO: Update the weights to include swap transaction's weight
        #[weight = 10_000 + T::DbWeight::get().writes(1)]
        pub fn claim_feeless_transaction(origin, stake_amount: T::Balance, call: <T as Config>::Call) -> dispatch::DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(stake_amount >= T::MinStakeAmount::get(), Error::<T>::StakeAmountTooSmall);
            ensure!(stake_amount <= T::Currency::free_balance(AssetId::POLKADEX,&who), Error::<T>::NotEnoughBalanceToStake);

            let mut stored_exts: ExtStore<T> = Self::get_next_block_txns();
            ensure!(stored_exts.store.len() < T::MaxAllowedTxns::get(), Error::<T>::NoMoreFeelessTxnsForThisBlock);

            // Update the moving average of stake amount
            let mut stake_moving_average: MovingAverage<T> = Self::get_stake_moving_average();
            stake_moving_average.update_stake_amount(&stake_amount);

            // Get current block number
            let current_block_number: T::BlockNumber = <frame_system::Pallet<T>>::block_number();

            // Store the staking record
            let mut staked_amount: StakeInfo<T> = Self::staked_users(who.clone());
            staked_amount.staked_amount += stake_amount;
            staked_amount.unlocking_block = current_block_number + T::MinStakePeriod::get();


            // Store the transactions randomize and execute on next block's initialize
            stored_exts.store.push(Ext{
                call,
                origin
            });

            <StakedUsers<T>>::put(who.clone(),staked_amount);
            <TxnsForNextBlock<T>>::put(stored_exts);
            <StakeMovingAverage<T>>::put(stake_moving_average);
            Self::deposit_event(RawEvent::FeelessExtrinsicAccepted(call));
            Ok(())
        }
    }
}

