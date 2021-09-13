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

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// https://substrate.dev/docs/en/knowledgebase/runtime/frame
use frame_support::{decl_error, decl_event, decl_module, decl_storage};
use frame_support::pallet_prelude::*;
use frame_support::traits::{Currency, LockableCurrency, WithdrawReasons};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::traits::{BlockNumberProvider, Zero};
use frame_support::traits::fungible::Mutate;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod benchmarking;

const MIGRATION_LOCK: frame_support::traits::LockIdentifier = *b"pdexlock";


/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Config: frame_system::Config + pallet_balances::Config + pallet_sudo::Config {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
    /// Lock Period
    type LockPeriod: Get<<Self as frame_system::Config>::BlockNumber>;
    /// Weight Info for PDEX migration
    type WeightInfo;
}

decl_storage! {
    trait Store for Module<T: Config> as NativePDEXMigration {
        /// List of relayers who can relay data from Ethereum
        Relayers get(fn relayers): map hasher(blake2_128_concat) T::AccountId => bool;
        /// Flag that enables the migration
        Operational get(fn operational) config(operation_status): bool;
        /// Maximum Mintable tokens
        MintableTokens get(fn mintable_tokens) config(max_tokens): T::Balance;
        /// Locked Token holders
        LockedTokenHolders get(fn locked_holders):  map hasher(blake2_128_concat) T::AccountId => Option<T::BlockNumber>;
        /// Processed Eth Burn Transactions
        EthTxns get(fn eth_txs): map hasher(blake2_128_concat) T::Hash => u32;
    }
}

// Pallets use events to inform users when important changes are made.
// https://substrate.dev/docs/en/knowledgebase/runtime/events
decl_event!(
    pub enum Event<T>
    where
        AccountId = <T as frame_system::Config>::AccountId,
        Balance = <T as pallet_balances::Config>::Balance,
    {
        RelayerStatusUpdated(AccountId,bool),
        NotOperational,
        NativePDEXMintedAndLocked(AccountId,AccountId,Balance),
        RevertedMintedTokens(AccountId),
    }
);

// Errors inform users that something went wrong.
decl_error! {
    pub enum Error for Module<T: Config> {
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

        #[weight = 10000]
        pub fn set_migration_operational_status(origin, status: bool) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Operational::put(status);
            Ok(Pays::No.into())
        }

        #[weight = 10000]
        pub fn set_relayer_status(origin, relayer: T::AccountId, status: bool) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Relayers::<T>::insert(&relayer,status);
            Self::deposit_event(RawEvent::RelayerStatusUpdated(relayer,status));
            Ok(Pays::No.into())
        }

        #[weight = 10000]
        pub fn mint(origin, beneficiary: T::AccountId, amount: T::Balance, eth_tx: T::Hash) -> DispatchResultWithPostInfo {
            let relayer = ensure_signed(origin)?;
            if Self::operational(){
                Self::process_migration(relayer,beneficiary,amount,eth_tx)?;
                Ok(Pays::No.into())
            }else{
                Err(Error::<T>::NotOperational)?
            }
        }

        #[weight = 10000]
        pub fn unlock(origin) -> DispatchResultWithPostInfo {
            let beneficiary = ensure_signed(origin)?;
            if Self::operational(){
                Self::process_unlock(beneficiary)?;
                Ok(Pays::No.into())
            }else{
                Err(Error::<T>::NotOperational)?
            }
        }

        #[weight = 10000]
        pub fn remove_minted_tokens(origin, beneficiary: T::AccountId) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;
            Self::remove_fradulent_tokens(beneficiary)?;
            Ok(Pays::No.into())
        }
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
        Self::deposit_event(RawEvent::RevertedMintedTokens(beneficiary));
        Ok(())
    }
    pub fn process_migration(relayer: T::AccountId,
                             beneficiary: T::AccountId,
                             amount: T::Balance,
                             eth_hash: T::Hash) -> Result<(), Error<T>> {
        let relayer_status = Relayers::<T>::get(&relayer);

        if relayer_status {
            let mut mintable_tokens = Self::mintable_tokens();
            if amount <= mintable_tokens {
                let mut num_votes = EthTxns::<T>::get(&eth_hash);
                num_votes = num_votes + 1;
                if num_votes == 3 { // We need all three relayers to agree on this burn transaction
                    // Mint tokens
                    let _positive_imbalance = pallet_balances::Pallet::<T>::deposit_creating(&beneficiary, amount);
                    // Lock tokens for 28 days
                    pallet_balances::Pallet::<T>::set_lock(MIGRATION_LOCK,
                                                           &beneficiary,
                                                           amount,
                                                           WithdrawReasons::FEE);
                    let current_blocknumber: T::BlockNumber = frame_system::Pallet::<T>::current_block_number();
                    LockedTokenHolders::<T>::insert(beneficiary.clone(), current_blocknumber);
                    // Reduce possible mintable tokens
                    mintable_tokens = mintable_tokens - amount;
                    // Set reduced mintable tokens
                    MintableTokens::<T>::put(mintable_tokens);
                    EthTxns::<T>::insert(&eth_hash, num_votes);
                    Self::deposit_event(RawEvent::NativePDEXMintedAndLocked(relayer, beneficiary, amount));
                } else {
                    EthTxns::<T>::insert(&eth_hash, num_votes);
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
