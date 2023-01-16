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

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use sp_std::{collections::btree_set::BTreeSet, vec::Vec};

	use frame_support::{
		dispatch::fmt::Debug,
		log,
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, ReservableCurrency},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AccountIdConversion, Zero},
		SaturatedConversion,
	};

	use thea_primitives::{BLSPublicKey, Payload};

	pub type Network = u32;

	#[derive(Encode, Decode, Clone, Debug, MaxEncodedLen, TypeInfo, Copy)]
	pub struct ApprovedDeposit {
		pub asset_id: u128,
		pub amount: u128,
		pub tx_hash: sp_core::H256,
	}

	#[derive(Encode, Decode, Clone, Debug, TypeInfo)]
	pub struct ApprovedWithdraw {
		pub asset_id: u128,
		pub amount: u128,
		pub network: u8,
		pub beneficiary: Vec<u8>,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config: frame_system::Config + asset_handler::pallet::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
		/// Thea PalletId
		#[pallet::constant]
		type TheaPalletId: Get<PalletId>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Active Relayers BLS Keys for a given Netowkr
	#[pallet::storage]
	#[pallet::getter(fn get_relayers_key_vector)]
	pub(super) type RelayersBLSKeyVector<T: Config> = StorageMap<
		_,
		frame_support::Blake2_128Concat,
		u8,
		BoundedVec<BLSPublicKey, ConstU32<1000>>,
		ValueQuery,
	>;

	/// Approved Deposits
	#[pallet::storage]
	#[pallet::getter(fn get_approved_deposits)]
	pub(super) type ApprovedDeposits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ApprovedDeposit, ConstU32<100>>,
		OptionQuery,
	>;

	/// Pending Withdrawals for batch completion
	#[pallet::storage]
	#[pallet::getter(fn pending_withdrawals)]
	pub(super) type PendingWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		Network,
		BoundedVec<ApprovedWithdraw, ConstU32<10>>,
		ValueQuery,
	>;

	/// Withdrawal Fees for each network
	#[pallet::storage]
	#[pallet::getter(fn witdrawal_fees)]
	pub(super) type WithdrawalFees<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u128, OptionQuery>;

	/// Withdrawal batches ready for sigining
	#[pallet::storage]
	#[pallet::getter(fn ready_withdrawals)]
	pub(super) type ReadyWithdrawls<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Blake2_128Concat,
		Network,
		BoundedVec<ApprovedWithdraw, ConstU32<10>>,
		ValueQuery,
	>;

	/// Withdrawal nonces for each network
	#[pallet::storage]
	#[pallet::getter(fn withdrawal_nonces)]
	pub(super) type WithdrawalNonces<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	/// Accounts which have a pending deposit
	#[pallet::storage]
	#[pallet::getter(fn accounts_with_pending_deposits)]
	pub(super) type AccountWithPendingDeposits<T: Config> =
		StorageValue<_, BTreeSet<T::AccountId>, ValueQuery>;

	/// Asset id to network mapping
	/// u128 => u8
	#[pallet::storage]
	#[pallet::getter(fn asset_id_to_network)]
	pub(super) type AssetIdToNetworkMapping<T: Config> =
		StorageMap<_, Blake2_128Concat, u128, Network, OptionQuery>;

	/// Deposit Nonce for Thea Deposits
	#[pallet::storage]
	#[pallet::getter(fn get_deposit_nonce)]
	pub(super) type DepositNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Deposit Approved event ( recipient, asset_id, amount, tx_hash(foreign chain))
		DepositApproved(u8, T::AccountId, u128, u128, sp_core::H256),
		// Deposit claimed event ( recipient, number of deposits claimed )
		DepositClaimed(T::AccountId, u128, u128, sp_core::H256),
		// Withdrawal Queued ( beneficiary, assetId, amount )
		WithdrawalQueued(T::AccountId, Vec<u8>, u128, u128, u32),
		// Withdrawal Ready (Network id, Nonce )
		WithdrawalReady(Network, u32),
		// Withdrawal Executed (Nonce, network, Tx hash )
		WithdrawalExecuted(u32, Network, sp_core::H256),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		// Nonce does not match
		DepositNonceError,
		// Amount cannot be zero
		AmountCannotBeZero,
		// Asset has not been registered
		AssetNotRegistered,
		// BLS Aggregate signature failed
		BLSSignatureVerificationFailed,
		// Beneficiary Size too long
		BeneficiaryTooLong,
		// Unable to find mapping between asset id to network
		UnableFindNetworkForAssetId,
		// Too many withdrawals in queue,
		WithdrawalNotAllowed,
		// Withdrawal fee is not configured this network
		WithdrawalFeeConfigNotFound,
		// No approved deposits for the provided account
		NoApprovedDeposit,
	}

	// Hooks for Thea Pallet are defined here
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_n: BlockNumberFor<T>, mut remaining_weight: Weight) -> Weight {
			// TODO: Calculate proper weight for single claim call on on_idle
			let single_claim_weight: Weight = 100_000_000;

			if remaining_weight < single_claim_weight {
				// We need enough weight for at least one claim process if not it's a no-op
				return remaining_weight
			}

			let mut accounts = <AccountWithPendingDeposits<T>>::get();
			if accounts.is_empty() {
				return remaining_weight
			}

			while let Some(account) = accounts.pop_first() {
				if let Some(mut pending_deposits) = <ApprovedDeposits<T>>::get(&account) {
					// FIXME: This leads to an infinite loop if execute_deposit fails
					while let Some(deposit) = pending_deposits.pop() {
						if let Err(err) = Self::execute_deposit(deposit, &account) {
							// Force push is fine as it was part of the bounded vec
							pending_deposits.force_push(deposit);
							// We can't do much here other than to log an error.
							log::error!(target:"runtime::thea::on_idle","Error while claiming deposit on idle: user: {:?}, Err: {:?}",account,err);
						}
						// reduce the remaining_weight
						remaining_weight = remaining_weight.saturating_sub(single_claim_weight);
						if remaining_weight.is_zero() {
							break
						}
					}

					if !pending_deposits.is_empty() {
						<ApprovedDeposits<T>>::insert(&account, pending_deposits);
						accounts.insert(account);
					}
				}
			}

			<AccountWithPendingDeposits<T>>::put(accounts);
			remaining_weight
		}
	}

	// Extrinsics for Thea Pallet are defined here
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// A Deposit transaction is called by the nodes with an aggregated BLS Signature
		///
		/// # Parameters
		///
		/// * `origin`: Active relayer
		/// * `bit_map`: The bit map of current relayer set that have signed the Deposit Transaction
		/// * `bls_signature`: The aggregated signature of majority of relayers in current active
		/// * `payload`: Deposit payload that has been signed by the current active relayer set
		#[pallet::weight(1000)]
		pub fn approve_deposit(
			origin: OriginFor<T>,
			bit_map: u128,
			bls_signature: [u8; 96],
			payload: Payload<T::AccountId>,
		) -> DispatchResult {
			ensure_signed(origin)?;
			ensure!(payload.amount > 0, Error::<T>::AmountCannotBeZero);
			// Fetch Deposit Nonce
			let nonce = <DepositNonce<T>>::get(payload.network_id.saturated_into::<Network>());
			ensure!(payload.deposit_nonce == nonce + 1, Error::<T>::DepositNonceError);
			// Ensure assets are registered
			ensure!(
				asset_handler::pallet::TheaAssets::<T>::contains_key(payload.asset_id),
				Error::<T>::AssetNotRegistered
			);

			// Fetch current active relayer set BLS Keys
			let current_active_relayer_set = Self::get_relayers_key_vector(payload.network_id);

			// Call host function with current_active_relayer_set, signature, bit_map, verify nonce
			ensure!(
				thea_primitives::thea_ext::bls_verify(
					&bls_signature,
					bit_map,
					&payload.encode(),
					&current_active_relayer_set.into_inner()
				),
				Error::<T>::BLSSignatureVerificationFailed
			);

			// Update deposit Nonce
			<DepositNonce<T>>::insert(payload.network_id.saturated_into::<Network>(), nonce + 1);

			// Update Storage item
			let approved_deposit = ApprovedDeposit {
				asset_id: payload.asset_id,
				amount: payload.amount,
				tx_hash: payload.tx_hash,
			};
			if <ApprovedDeposits<T>>::contains_key(&payload.who) {
				<ApprovedDeposits<T>>::mutate(payload.who.clone(), |bounded_vec| {
					if let Some(inner_bounded_vec) = bounded_vec {
						inner_bounded_vec.try_push(approved_deposit).unwrap();
					}
				});
			} else {
				let mut my_vec: BoundedVec<ApprovedDeposit, ConstU32<100>> = Default::default();
				if let Ok(()) = my_vec.try_push(approved_deposit) {
					<ApprovedDeposits<T>>::insert::<
						T::AccountId,
						BoundedVec<ApprovedDeposit, ConstU32<100>>,
					>(payload.who.clone(), my_vec);
					<AccountWithPendingDeposits<T>>::mutate(|accounts| {
						accounts.insert(payload.who.clone())
					});
				}
			}

			// Emit event
			Self::deposit_event(Event::<T>::DepositApproved(
				payload.network_id,
				payload.who,
				payload.asset_id,
				payload.amount,
				payload.tx_hash,
			));
			Ok(())
		}

		/// Manually claim an approved deposit
		///
		/// # Parameters
		///
		/// * `origin`: User
		/// * `num_deposits`: Number of deposits to claim from available deposits,
		/// (it's used to parametrise the weight of this extrinsic)
		#[pallet::weight(1000)]
		pub fn claim_deposit(origin: OriginFor<T>, num_deposits: u32) -> DispatchResult {
			let user = ensure_signed(origin)?;

			if let Some(mut deposits) = <ApprovedDeposits<T>>::get(&user) {
				let length: u32 = if deposits.len().saturated_into::<u32>() <= num_deposits {
					deposits.len().saturated_into()
				} else {
					num_deposits
				}
				.saturated_into();

				for _ in 0..length {
					if let Some(deposit) = deposits.pop() {
						if let Err(err) = Self::execute_deposit(deposit, &user) {
							// Force push is fine as it will have the capacity.
							deposits.force_push(deposit);
							// Save it back on failure
							<ApprovedDeposits<T>>::insert(&user, deposits.clone());
							return Err(err)
						}
					} else {
						break
					}
				}

				if !deposits.is_empty() {
					// If pending deposits are available, save it back
					<ApprovedDeposits<T>>::insert(&user, deposits)
				} else {
					<AccountWithPendingDeposits<T>>::mutate(|accounts| accounts.remove(&user));
				}
			} else {
				return Err(Error::<T>::NoApprovedDeposit.into())
			}

			Ok(())
		}

		/// Extrinsic to update withdrawal completion status by relayer
		///
		/// # Parameters
		///
		/// * `origin`: Any relayer
		/// * `withdrawal_nonce`: Withdrawal Nonce
		/// * `network`: Network id
		/// * `tx_hash`: Vec<u8>
		/// * `bit_map`: Bitmap of Thea relayers
		/// * `bls_signature`: BLS signature of relayers
		#[pallet::weight(1000)]
		pub fn batch_withdrawal_complete(
			origin: OriginFor<T>,
			withdrawal_nonce: u32,
			network: Network,
			tx_hash: sp_core::H256,
			_bit_map: u128,
			_bls_signature: [u8; 96],
		) -> DispatchResult {
			ensure_signed(origin)?;

			// TODO: This will be refactored when work on withdrawal begins
			<ReadyWithdrawls<T>>::take(network, withdrawal_nonce);

			Self::deposit_event(Event::<T>::WithdrawalExecuted(withdrawal_nonce, network, tx_hash));
			Ok(())
		}

		/// Initiate the withdraw for user
		///
		/// # Parameters
		///
		/// * `origin`: User
		/// * `asset_id`: Asset id
		/// * `amount`: Amount of asset to withdraw
		/// * `beneficiary`: beneficiary of the withdraw
		/// * `pay_for_remaining`: user is ready to pay for remaining pending withdrawal for quick
		///   withdrawal
		#[pallet::weight(1000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			// Put a soft limit of size of beneficiary vector to avoid spam
			ensure!(beneficiary.len() <= 100, Error::<T>::BeneficiaryTooLong);

			// Find native network of this asset
			#[allow(clippy::unnecessary_lazy_evaluations)]
			// TODO: This will be refactored when work on withdrawal so not fixing clippy suggestion
			let network = <AssetIdToNetworkMapping<T>>::get(asset_id)
				.ok_or_else(|| Error::<T>::UnableFindNetworkForAssetId)?;

			let withdrawal_nonce = <WithdrawalNonces<T>>::get(network);

			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(network);

			// Ensure pending withdrawals have space for a new withdrawal
			ensure!(pending_withdrawals.is_full(), Error::<T>::WithdrawalNotAllowed);

			#[allow(clippy::unnecessary_lazy_evaluations)]
			// TODO: This will be refactored when work on withdrawal so not fixing clippy suggestion
			let mut total_fees = <WithdrawalFees<T>>::get(network)
				.ok_or_else(|| Error::<T>::WithdrawalFeeConfigNotFound)?;

			if pay_for_remaining {
				// User is ready to pay for remaining pending withdrawal for quick withdrawal
				let extra_withdrawals_available = 10usize.saturating_sub(pending_withdrawals.len());
				total_fees = total_fees.saturating_add(
					total_fees.saturating_mul(extra_withdrawals_available.saturated_into()),
				)
			}

			// Pay the fees
			<T as Config>::Currency::transfer(
				&user,
				&Self::thea_account(),
				total_fees.saturated_into(),
				ExistenceRequirement::KeepAlive,
			)?;

			// TODO[#610]: Update Thea Staking pallet about fees collected

			// Burn assets
			asset_handler::pallet::Pallet::<T>::burn_thea_asset(asset_id, user.clone(), amount)?;

			let withdrawal = ApprovedWithdraw {
				asset_id,
				amount: amount.saturated_into(),
				network: network.saturated_into(),
				beneficiary: beneficiary.clone(),
			};

			if let Err(()) = pending_withdrawals.try_push(withdrawal) {
				// This should not fail because of is_full check above
			}

			if pending_withdrawals.is_full() | pay_for_remaining {
				// If it is full then we move it to ready queue and update withdrawal nonce
				let withdrawal_nonce = <WithdrawalNonces<T>>::get(network);
				<ReadyWithdrawls<T>>::insert(
					network,
					withdrawal_nonce,
					pending_withdrawals.clone(),
				);
				<WithdrawalNonces<T>>::insert(network, withdrawal_nonce.saturating_add(1));
				Self::deposit_event(Event::<T>::WithdrawalReady(network, withdrawal_nonce));
				pending_withdrawals = BoundedVec::default();
			}
			<PendingWithdrawals<T>>::insert(network, pending_withdrawals);
			Self::deposit_event(Event::<T>::WithdrawalQueued(
				user,
				beneficiary,
				asset_id,
				amount,
				withdrawal_nonce,
			));
			Ok(())
		}
	}

	// Helper Functions for Thea Pallet
	impl<T: Config> Pallet<T> {
		pub fn thea_account() -> T::AccountId {
			T::TheaPalletId::get().into_account_truncating()
		}

		pub fn execute_deposit(
			deposit: ApprovedDeposit,
			recipient: &T::AccountId,
		) -> Result<(), DispatchError> {
			asset_handler::pallet::Pallet::<T>::mint_thea_asset(
				deposit.asset_id,
				recipient.clone(),
				deposit.amount,
			)?;
			// Emit event
			Self::deposit_event(Event::<T>::DepositClaimed(
				recipient.clone(),
				deposit.asset_id,
				deposit.amount,
				deposit.tx_hash,
			));
			Ok(())
		}
	}
}
