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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::Get,
	traits::{fungibles::Mutate, Currency, ExistenceRequirement},
	BoundedVec,
};
use frame_system::ensure_signed;
use polkadex_primitives::{assets::AssetId, OnChainEventsLimit};

use pallet_timestamp::{self as timestamp};
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
use sp_std::prelude::*;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const DEPOSIT_MAX: u128 = 1_000_000_000_000_000_000_000_000_000;
const TRADE_OPERATION_MIN_VALUE: u128 = 10000;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
	use core::fmt::Debug;
	// Import various types used to declare pallet in scope.
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_tracing::debug,
		storage::bounded_btree_map::BoundedBTreeMap,
		traits::{
			fungibles::{Create, Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ias_verify::{verify_ias_report, SgxStatus};
	use polkadex_primitives::{
		assets::AssetId,
		ocex::{AccountInfo, TradingPairConfig},
		snapshot::{EnclaveSnapshot, Fees},
		withdrawal::Withdrawal,
		AssetsLimit, ProxyLimit, SnapshotAccLimit, WithdrawalLimit, UNIT_BALANCE,
	};
	use rust_decimal::{prelude::ToPrimitive, Decimal};
	use sp_runtime::{
		traits::{IdentifyAccount, Verify},
		BoundedBTreeSet, SaturatedConversion,
	};
	use sp_std::vec::Vec;


	type WithdrawalsMap<T> = BoundedBTreeMap<
		<T as frame_system::Config>::AccountId,
		BoundedVec<Withdrawal<<T as frame_system::Config>::AccountId>, WithdrawalLimit>,
		SnapshotAccLimit,
	>;

	type EnclaveSnapshotType<T> = EnclaveSnapshot<
		<T as frame_system::Config>::AccountId,
		WithdrawalLimit,
		AssetsLimit,
		SnapshotAccLimit,
	>;

	pub struct AllowlistedTokenLimit;
	impl Get<u32> for AllowlistedTokenLimit {
		fn get() -> u32 {
			50 // TODO: Arbitrary value
		}
	}

	/// Our pallet's configuration trait. All our types and constants go in here. If the
	/// pallet is dependent on specific other pallets, then their configuration traits
	/// should be added to our implied traits list.
	///
	/// `frame_system::Config` should always be included.
	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Balances Pallet
		type NativeCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Assets Pallet
		type OtherAssets: Mutate<
				<Self as frame_system::Config>::AccountId,
				Balance = BalanceOf<Self>,
				AssetId = u128,
			> + Inspect<<Self as frame_system::Config>::AccountId>
			+ Create<<Self as frame_system::Config>::AccountId>;

		type Public: Clone
			+ PartialEq
			+ IdentifyAccount<AccountId = Self::AccountId>
			+ Debug
			+ parity_scale_codec::Codec
			+ Ord
			+ scale_info::TypeInfo;

		/// A matching `Signature` type.
		type Signature: Verify<Signer = Self::Public>
			+ Clone
			+ PartialEq
			+ Debug
			+ parity_scale_codec::Codec
			+ scale_info::TypeInfo;

	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);
	
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			(1000000 as Weight)
				.saturating_add(T::DbWeight::get().reads(2 as Weight))
				.saturating_add(T::DbWeight::get().writes(2 as Weight))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers a new account in orderbook
		#[pallet::weight(10_000)]
		pub fn register_main_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			Ok(())
		}
	}
	impl<T: Config> Pallet<T> {
		// clean-up function - should be called on each block
		#[allow(dead_code)]
		fn unregister_timed_out_enclaves() {
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FeesClaims {
			beneficiary: T::AccountId,
			snapshot_id: u32,
		},
	}

	// Vector of registered enclaves
	#[pallet::storage]
	#[pallet::getter(fn get_registered_enclaves)]
	pub(super) type RegisteredEnclavesA<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::Moment, ValueQuery>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	
	fn transfer_asset(
		payer: &T::AccountId,
		payee: &T::AccountId,
		amount: BalanceOf<T>,
		asset: AssetId,
	) -> DispatchResult {
		match asset {
			AssetId::polkadex => {
				T::NativeCurrency::transfer(
					payer,
					payee,
					amount.unique_saturated_into(),
					ExistenceRequirement::KeepAlive,
				)?;
			},
			AssetId::asset(id) => {
				T::OtherAssets::teleport(id, payer, payee, amount.unique_saturated_into())?;
			},
		}
		Ok(())
	}
}
