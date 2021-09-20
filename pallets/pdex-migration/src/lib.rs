#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;


#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode};
    use frame_support::pallet_prelude::*;
    use frame_support::traits::{Currency, LockableCurrency, WithdrawReasons};
    use frame_support::traits::fungible::Mutate;
    use frame_system::pallet_prelude::*;
    use sp_runtime::SaturatedConversion;
    use sp_runtime::traits::{BlockNumberProvider, Zero};
    use sp_std::vec::Vec;
    use sp_std::vec;

    const MIGRATION_LOCK: frame_support::traits::LockIdentifier = *b"pdexlock";

    #[derive(Encode, Decode, Debug)]
    pub struct BurnTxDetails<T: Config> {
        pub(crate) approvals: u16,
        pub(crate) approvers: Vec<T::AccountId>,
    }

    impl<T: Config> Default for BurnTxDetails<T> {
        fn default() -> Self {
            Self { approvals: 0, approvers: vec![] }
        }
    }

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    /// Configure the pallet by specifying the parameters and types on which it depends.
    pub trait Config: frame_system::Config + pallet_balances::Config + pallet_sudo::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Lock Period
        #[pallet::constant]
        type LockPeriod: Get<<Self as frame_system::Config>::BlockNumber>;
        /// Weight Info for PDEX migration
        type WeightInfo;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// List of relayers who can relay data from Ethereum
    #[pallet::storage]
    #[pallet::getter(fn relayers)]
    pub(super) type Relayers<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

    /// Flag that enables the migration
    #[pallet::storage]
    #[pallet::getter(fn operational)]
    pub(super) type Operational<T: Config> = StorageValue<_, bool, ValueQuery>;

    /// Maximum Mintable tokens
    #[pallet::storage]
    #[pallet::getter(fn mintable_tokens)]
    pub(super) type MintableTokens<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

    /// Locked Token holders
    #[pallet::storage]
    #[pallet::getter(fn locked_holders)]
    pub(super) type LockedTokenHolders<T: Config> = StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, OptionQuery>;

    /// Processed Eth Burn Transactions
    #[pallet::storage]
    #[pallet::getter(fn eth_txs)]
    pub(super) type EthTxns<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, BurnTxDetails<T>, ValueQuery>;

    // In FRAME v2.
    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub operational: bool,
        pub max_tokens: T::Balance,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { operational: false, max_tokens: 3_172_895u128.saturating_mul(1000_000_000_000u128).saturated_into() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            Operational::<T>::put(self.operational);
            MintableTokens::<T>::put(self.max_tokens.saturated_into::<T::Balance>());
        }
    }

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        RelayerStatusUpdated(T::AccountId, bool),
        NotOperational,
        NativePDEXMintedAndLocked(T::AccountId, T::AccountId, T::Balance),
        RevertedMintedTokens(T::AccountId),
        TokenBurnDetected(T::Hash, T::AccountId),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Migration is not operational yet
        NotOperational,
        /// Relayer is not registered
        UnknownRelayer,
        /// Invalid amount of tokens to mint
        InvalidMintAmount,
        /// This account has not minted any tokens.
        UnknownBeneficiary,
        /// Lock on minted tokens is not yet expired
        LiquidityRestrictions,
        /// Invalid Ethereum Tx Hash, Zero Hash
        InvalidTxHash,
        /// Given Eth Transaction is already processed
        AlreadyProcessedEthBurnTx,
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10000)]
        pub fn set_migration_operational_status(origin: OriginFor<T>, status: bool) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Operational::<T>::put(status);
            Ok(Pays::No.into())
        }

        #[pallet::weight(10000)]
        pub fn set_relayer_status(origin: OriginFor<T>, relayer: T::AccountId, status: bool) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Relayers::<T>::insert(&relayer, status);
            Self::deposit_event(Event::RelayerStatusUpdated(relayer, status));
            Ok(Pays::No.into())
        }

        #[pallet::weight(10000)]
        pub fn mint(origin: OriginFor<T>, beneficiary: T::AccountId, amount: T::Balance, eth_tx: T::Hash) -> DispatchResultWithPostInfo {
            let relayer = ensure_signed(origin)?;
            ensure!(eth_tx != T::Hash::default(), Error::<T>::InvalidTxHash);
            if Self::operational() {
                let burn_details = EthTxns::<T>::get(eth_tx);
                ensure!(!burn_details.approvers.contains(&relayer), Error::<T>::AlreadyProcessedEthBurnTx);
                Self::process_migration(relayer, beneficiary, amount, eth_tx, burn_details)?;
                Ok(Pays::No.into())
            } else {
                Err(Error::<T>::NotOperational)?
            }
        }

        #[pallet::weight(10000)]
        pub fn unlock(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
            let beneficiary = ensure_signed(origin)?;
            if Self::operational() {
                Self::process_unlock(beneficiary)?;
                Ok(Pays::No.into())
            } else {
                Err(Error::<T>::NotOperational)?
            }
        }

        #[pallet::weight(10000)]
        pub fn remove_minted_tokens(origin: OriginFor<T>, beneficiary: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::remove_fradulent_tokens(beneficiary)?;
            Ok(Pays::No.into())
        }
    }


    impl<T: Config> Pallet<T> {
        pub fn remove_fradulent_tokens(beneficiary: T::AccountId) -> Result<(), DispatchError> {
            LockedTokenHolders::<T>::take(&beneficiary);
            let locks = pallet_balances::Locks::<T>::get(&beneficiary);
            let mut amount_to_burn: T::Balance = T::Balance::zero();
            // Loop and find the migration lock
            for lock in locks {
                if lock.id == MIGRATION_LOCK {
                    amount_to_burn = lock.amount;
                    break;
                }
            }

            pallet_balances::Pallet::<T>::remove_lock(MIGRATION_LOCK, &beneficiary);
            // Burn the illegally minted tokens
            pallet_balances::Pallet::<T>::burn_from(&beneficiary, amount_to_burn)?;
            // Increment total mintable tokens
            let mut mintable_tokens = MintableTokens::<T>::get();
            mintable_tokens = mintable_tokens + amount_to_burn;
            MintableTokens::<T>::put(mintable_tokens);
            // Deposit event
            Self::deposit_event(Event::RevertedMintedTokens(beneficiary));
            Ok(())
        }
        pub fn process_migration(relayer: T::AccountId,
                                 beneficiary: T::AccountId,
                                 amount: T::Balance,
                                 eth_hash: T::Hash,
                                 mut burn_details: BurnTxDetails<T> ) -> Result<(), Error<T>> {
            let relayer_status = Relayers::<T>::get(&relayer);

            if relayer_status {
                let mut mintable_tokens = Self::mintable_tokens();
                if amount <= mintable_tokens {
                    burn_details.approvals = burn_details.approvals + 1;
                    burn_details.approvers.push(relayer.clone());
                    if burn_details.approvals == 3 { // We need all three relayers to agree on this burn transaction
                        // Mint tokens
                        let _positive_imbalance = pallet_balances::Pallet::<T>::deposit_creating(&beneficiary, amount);
                        let reasons = WithdrawReasons::TRANSFER;
                        // Lock tokens for 28 days
                        pallet_balances::Pallet::<T>::set_lock(MIGRATION_LOCK,
                                                               &beneficiary,
                                                               amount,
                                                               reasons);
                        let current_blocknumber: T::BlockNumber = frame_system::Pallet::<T>::current_block_number();
                        LockedTokenHolders::<T>::insert(beneficiary.clone(), current_blocknumber);
                        // Reduce possible mintable tokens
                        mintable_tokens = mintable_tokens - amount;
                        // Set reduced mintable tokens
                        MintableTokens::<T>::put(mintable_tokens);
                        EthTxns::<T>::insert(&eth_hash, burn_details);
                        Self::deposit_event(Event::NativePDEXMintedAndLocked(relayer, beneficiary, amount));
                    } else {
                        EthTxns::<T>::insert(&eth_hash, burn_details);
                        Self::deposit_event(Event::TokenBurnDetected(eth_hash, relayer));
                    }
                    Ok(())
                } else {
                    Err(Error::<T>::InvalidMintAmount)
                }
            } else {
                Err(Error::<T>::UnknownRelayer)
            }
        }


        pub fn process_unlock(beneficiary: T::AccountId) -> Result<(), Error<T>> {
            if let Some(locked_block) = LockedTokenHolders::<T>::take(&beneficiary) {
                if locked_block + T::LockPeriod::get() <= frame_system::Pallet::<T>::current_block_number() {
                    pallet_balances::Pallet::<T>::remove_lock(MIGRATION_LOCK, &beneficiary);
                    Ok(())
                } else {
                    LockedTokenHolders::<T>::insert(&beneficiary, locked_block);
                    Err(Error::<T>::LiquidityRestrictions)
                }
            } else {
                Err(Error::<T>::UnknownBeneficiary)
            }
        }
    }
}
