// This file is part of Polkadex.
//
// Copyright (c) 2021-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # PDEX Migration Pallet.
//!
//! The PDEX Migration Pallet used for migrating ERC20 PDEX to Native PDEX.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![deny(unused_crate_dependencies)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungible::Mutate,
			tokens::{Fortitude, Precision},
			Currency, Get, LockableCurrency, WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::*;
	use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{BlockNumberProvider, Saturating, Zero},
		SaturatedConversion,
	};

	const MIGRATION_LOCK: frame_support::traits::LockIdentifier = *b"pdexlock";

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen)]
	#[scale_info(skip_type_params(MaxRelayers))]
	#[codec(mel_bound(AccountId: MaxEncodedLen))]
	pub struct BurnTxDetails<AccountId, MaxRelayers: Get<u32>> {
		pub(crate) approvals: u16,
		pub(crate) approvers: BoundedVec<AccountId, MaxRelayers>,
	}

	impl<AccountId, MaxRelayers: Get<u32>> Default for BurnTxDetails<AccountId, MaxRelayers> {
		fn default() -> Self {
			Self { approvals: 0, approvers: BoundedVec::default() }
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config: frame_system::Config + pallet_balances::Config + pallet_sudo::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Max Number of relayers
		#[pallet::constant]
		type MaxRelayers: Get<u32>;
		/// Lock Period
		#[pallet::constant]
		type LockPeriod: Get<<Self as frame_system::Config>::BlockNumber>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// List of relayers who can relay data from Ethereum
	#[pallet::storage]
	#[pallet::getter(fn relayers)]
	pub(super) type Relayers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

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
	pub(super) type LockedTokenHolders<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, OptionQuery>;

	/// Processed Eth Burn Transactions
	#[pallet::storage]
	#[pallet::getter(fn eth_txs)]
	pub(super) type EthTxns<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::Hash,
		BurnTxDetails<T::AccountId, T::MaxRelayers>,
		ValueQuery,
	>;

	// In FRAME v2.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub operational: bool,
		pub max_tokens: T::Balance,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self {
				operational: false,
				max_tokens: 3_172_895u128.saturating_mul(1_000_000_000_000u128).saturated_into(),
			}
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
		/// BoundedVec limit reached
		RelayerLimitReached,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Sets migration operational status.
		///
		/// # Parameters
		///
		/// * `status`: `bool` to define if bridge enabled or disabled.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(0)]
		pub fn set_migration_operational_status(
			origin: OriginFor<T>,
			status: bool,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Operational::<T>::put(status);
			Ok(Pays::No.into())
		}

		/// Updates relayer operational status.
		///
		/// # Parameters
		///
		/// * `relayer`: Relayer account identifier.
		/// * `status`: Operational or not.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(1)]
		pub fn set_relayer_status(
			origin: OriginFor<T>,
			relayer: T::AccountId,
			status: bool,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			Relayers::<T>::insert(&relayer, status);
			Self::deposit_event(Event::RelayerStatusUpdated(relayer, status));
			Ok(Pays::No.into())
		}

		/// Increases the balance of `who` by `amount`.
		///
		/// # Parameters
		///
		/// * `beneficiary`: Account on which balance should be increased.
		/// * `amount`: Amount on which balance should be increased.
		/// * `eth_tx`: Ethereum Tx Hash.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(2)]
		pub fn mint(
			origin: OriginFor<T>,
			beneficiary: T::AccountId,
			amount: T::Balance,
			eth_tx: T::Hash,
		) -> DispatchResultWithPostInfo {
			let relayer = ensure_signed(origin)?;
			ensure!(eth_tx != T::Hash::default(), Error::<T>::InvalidTxHash);
			if Self::operational() {
				let mut burn_details = EthTxns::<T>::get(eth_tx);
				ensure!(
					!burn_details.approvers.contains(&relayer),
					Error::<T>::AlreadyProcessedEthBurnTx
				);
				Self::process_migration(relayer, beneficiary, amount, eth_tx, &mut burn_details)?;
				Ok(Pays::No.into())
			} else {
				Err(Error::<T>::NotOperational)?
			}
		}

		/// Removes lock from the balance.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(3)]
		pub fn unlock(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let beneficiary = ensure_signed(origin)?;
			if Self::operational() {
				Self::process_unlock(beneficiary)?;
				Ok(Pays::No.into())
			} else {
				Err(Error::<T>::NotOperational)?
			}
		}

		/// Removes minted tokens locked in the migration process.
		///
		/// # Parameters
		///
		/// * `beneficiary`: Tokens holder.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(4)]
		pub fn remove_minted_tokens(
			origin: OriginFor<T>,
			beneficiary: T::AccountId,
		) -> DispatchResultWithPostInfo {
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
					break
				}
			}

			pallet_balances::Pallet::<T>::remove_lock(MIGRATION_LOCK, &beneficiary);
			// Burn the illegally minted tokens
			pallet_balances::Pallet::<T>::burn_from(
				&beneficiary,
				amount_to_burn,
				Precision::Exact,
				Fortitude::Polite,
			)?;
			// Increment total mintable tokens
			let mut mintable_tokens = MintableTokens::<T>::get();
			mintable_tokens += amount_to_burn;
			MintableTokens::<T>::put(mintable_tokens);
			// Deposit event
			Self::deposit_event(Event::RevertedMintedTokens(beneficiary));
			Ok(())
		}

		/// Executes tokens migration.
		pub fn process_migration(
			relayer: T::AccountId,
			beneficiary: T::AccountId,
			amount: T::Balance,
			eth_hash: T::Hash,
			burn_details: &mut BurnTxDetails<T::AccountId, T::MaxRelayers>,
		) -> Result<(), Error<T>> {
			let relayer_status = Relayers::<T>::get(&relayer);

			if relayer_status {
				let mut mintable_tokens = Self::mintable_tokens();
				if amount <= mintable_tokens {
					burn_details.approvals += 1;
					ensure!(
						burn_details.approvers.try_push(relayer.clone()).is_ok(),
						Error::RelayerLimitReached
					);
					if burn_details.approvals == 3 {
						// We need all three relayers to agree on this burn transaction
						// Mint tokens
						let _positive_imbalance =
							pallet_balances::Pallet::<T>::deposit_creating(&beneficiary, amount);
						let reasons = WithdrawReasons::TRANSFER;
						// Loads the previous locked balance for migration if any, else return zero
						let previous_balance: T::Balance =
							Self::previous_locked_balance(&beneficiary);
						// Lock tokens for 28 days
						pallet_balances::Pallet::<T>::set_lock(
							MIGRATION_LOCK,
							&beneficiary,
							amount.saturating_add(previous_balance),
							reasons,
						);
						let current_blocknumber: T::BlockNumber =
							frame_system::Pallet::<T>::current_block_number();
						LockedTokenHolders::<T>::insert(beneficiary.clone(), current_blocknumber);
						// Reduce possible mintable tokens
						mintable_tokens -= amount;
						// Set reduced mintable tokens
						MintableTokens::<T>::put(mintable_tokens);
						EthTxns::<T>::insert(eth_hash, burn_details);
						Self::deposit_event(Event::NativePDEXMintedAndLocked(
							relayer,
							beneficiary,
							amount,
						));
					} else {
						EthTxns::<T>::insert(eth_hash, burn_details);
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

		/// Removes migration lock from `beneficiary` account.
		///
		/// # Parameters
		///
		/// * `beneficiary`: Account to remove lock from.
		pub fn process_unlock(beneficiary: T::AccountId) -> Result<(), Error<T>> {
			if let Some(locked_block) = LockedTokenHolders::<T>::take(&beneficiary) {
				if locked_block + T::LockPeriod::get() <=
					frame_system::Pallet::<T>::current_block_number()
				{
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

		/// Provides balance of previously locked amount on the requested account.
		///
		/// # Parameters
		///
		/// * `who`: Account identifier.
		pub fn previous_locked_balance(who: &T::AccountId) -> T::Balance {
			let mut prev_locked_amount: T::Balance = T::Balance::zero();

			let locks = pallet_balances::Locks::<T>::get(who);
			// Loop is fine, since pallet_balances guarantee that it is not more than MAXLOCKS
			for lock in locks {
				if lock.id == MIGRATION_LOCK {
					prev_locked_amount = lock.amount;
				}
			}
			prev_locked_amount
		}
	}
}
