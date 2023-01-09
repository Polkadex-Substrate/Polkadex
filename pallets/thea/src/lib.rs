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
	use frame_support::{
		dispatch::fmt::Debug,
		log,
		pallet_prelude::*,
		traits::{
			tokens::fungibles::{Create, Inspect, Mutate},
			Currency, ExistenceRequirement, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{H160, U256};
	use sp_io::hashing::keccak_256;
	use sp_runtime::{
		traits::{One, Saturating, UniqueSaturatedInto},
		BoundedBTreeSet, SaturatedConversion,
	};
	use sp_std::{vec, vec::Vec};
	use thea_primitives::BLSPublicKey;

	#[derive(Encode, Decode, Clone, Debug, MaxEncodedLen, TypeInfo)]
	pub struct ApprovedDeposit {
		pub asset_id: u128,
		pub amount: u128,
	}

	#[derive(Encode, Decode, Clone, MaxEncodedLen, TypeInfo, PartialEq, Debug)]
	pub struct Payload<AccountId> {
		pub network_id: u8,
		pub who: AccountId,
		pub tx_hash: sp_core::H256,
		pub asset_id: u128,
		pub amount: u128,
		pub deposit_nonce: u32,
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
	pub struct Pallet<T>(_);

	// /// Set ID of the current active relayer set
	// #[pallet::storage]
	// #[pallet::getter(fn get_current_active_relayer_set_id)]
	// pub(super) type CurrentActiveRelayerSetId<T: Config> =
	// StorageMap<_, Blake2_128Concat, u8, u32, OptionQuery>;

	/// Active Relayers BLS Keys for a given Netowkr
	#[pallet::storage]
	#[pallet::getter(fn get_relayers_key_vector)]
	pub(super) type RelayersBLSKeyVector<T: Config> = StorageMap<
		_,
		frame_support::Blake2_128Concat,
		u8,
		BoundedVec<BLSPublicKey, ConstU32<1000>>,
		OptionQuery,
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

	#[pallet::storage]
	#[pallet::getter(fn get_deposit_nonce)]
	pub(super) type DepositNonce<T: Config> = StorageMap<_, Blake2_128Concat, u8, u32, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Deposit Approved event ( recipient, asset_id, amount, tx_hash(foreign chain))
		DepositApproved(T::AccountId, u128, u128, sp_core::H256),
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
	}

	// Hooks for Thea Pallet are defined here
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Extrinsics for Thea Pallet are defined here
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// A Deposit transaction is called by the nodes with an aggregated BLS Signature
		///
		/// # Parameters
		///
		/// * `origin`: Active relayer
		/// * `network_id`: id of the foreign chain network
		/// * `bit_map`: The bit map of current relayer set that have signed the Deposit Transaction
		/// * `recipient`: The address of the user who initiated a deposit
		/// * `tx_hash`: Hash of the transaction on the foreign chain
		/// * `asset_id`: The asset id of the asset being deposited
		/// * `amount`: The amount of assets that have been deposited in foreign chain
		/// * `bls_signature`: The aggregated signature of majority of relayers in current active
		///   relayer set
		#[pallet::weight(1000)]
		pub fn approve_deposit(
			origin: OriginFor<T>,
			bit_map: u128,
			bls_signature: [u8; 96],
			payload: Payload<T::AccountId>,
		) -> DispatchResult {
			ensure!(payload.amount > 0, Error::<T>::AmountCannotBeZero);
			// Fetch Deposit Nonce
			let nonce = <DepositNonce<T>>::get(payload.network_id);
			ensure!(payload.deposit_nonce == nonce + 1, Error::<T>::DepositNonceError);
			// ensure!(asset_handler::pallet::TheaAssets::<T>::contains_key(payload.asset_id),
			// Error::<T>::AssetNotRegistered);

			// Fetch current active relayer set BLS Keys
			let current_active_relayer_set =
				Self::get_relayers_key_vector(payload.network_id).unwrap();

			// Call host function with current_active_relayer_set, signature, bit_map, verify nonce
			// TODO: @gautham
			// Host Function Steps
			// Step 1: Get Payload, Signature, BLS Keys Vector
			// Step 2: Create Aggregate BLS Public Key
			// Step 3: Verify Aggregate Signature
			ensure!(
				thea_primitives::thea_ext::foo(
					bls_signature,
					bit_map,
					payload.encode(),
					current_active_relayer_set.into_inner()
				),
				Error::<T>::BLSSignatureVerificationFailed
			);

			// Update deposit Nonce
			<DepositNonce<T>>::insert(payload.network_id, nonce + 1);

			// Update Storage item
			let approved_deposit =
				ApprovedDeposit { asset_id: payload.asset_id, amount: payload.amount };
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
				}
			}

			// Emit event
			Self::deposit_event(Event::<T>::DepositApproved(
				payload.who,
				payload.asset_id,
				payload.amount,
				payload.tx_hash,
			));
			Ok(())
		}
	}

	// Helper Functions for Thea Pallet
	impl<T: Config> Pallet<T> {}
}
