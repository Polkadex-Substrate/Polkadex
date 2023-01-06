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

	pub struct ApprovedDeposit<T:Config>{
		pub who: T::AccountId,
		pub asset_id: u128,
		pub amount: u128
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

	/// Set ID of the current active relayer set
	#[pallet::storage]
	#[pallet::getter(fn get_current_active_relayer_set_id)]
	pub(super) type CurrentActiveRelayerSetId<T: Config> =
	StorageValue<_, u32, ValueQuery>;

	/// Active Relayers BLS Keys for a given Netowkr
	#[pallet::storage]
	#[pallet::getter(fn get_relayers_key_vector)]
	pub(super) type RelayersBLSKeyVector<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u8,
		u32,
		BoundedVec<[u8; 64], ConstU32<100>>,
		ValueQuery,
	>;

	/// Approved Deposits
	#[pallet::storage]
	#[pallet::getter(fn get_approved_deposits)]
	pub(super) type ApprovedDeposits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ApprovedDeposits<T>, ConstU32<100>>,
		ValueQuery,
	>;


	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		// Deposit Approved event ( recipient, asset_id, amount, tx_hash(foreign chain))
		DepositApproved(T::AccountId, u128, u128, H256)
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {}

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
		/// * `bls_signature`: The aggregated signature of majority of relayers in current active relayer set
		#[pallet::weight(1000)]
		pub fn approve_deposit(origin: OriginFor<T>, network_id: u8, bit_map: BoundedVec<u8, ConstU32<1000>>, recipient: T::AccountId, tx_hash: sp_core::H256, asset_id: u128, amount: u128, bls_signature: [u8;96]) -> DispatchResult {
			// Step 1: Check if Amount is above zero
			// Step 2: Check if Asset is valid asset (If not then the caller can be penalized)
			// Step 3: Check if Recipient has an existential deposit ( If not then it will be covered in another issue)
			// Step 4: Fetch the current active relayer set number ( Staking Pallet )
			// Step 5; Fetch the current active relayer set bls keys vector ( Staking Pallet )
			// Step 6: Call a host function with (bls keys vector, aggregated signature, bit map)
			// Step 7: If signature checks out, Mint the said amount to the recipient
			// Step 8: Emit an event for Frontend to receive
			Ok(())
		}


	}

	// Helper Functions for Thea Pallet
	impl<T: Config> Pallet<T> {}
}
