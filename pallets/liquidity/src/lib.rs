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

use frame_support::{dispatch::DispatchResult, traits::Currency};
use pallet_timestamp::{self as timestamp};
use sp_std::prelude::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

//constant proxy value
const PROXY_ACCOUNT_BYTES: [u8; 32] = [0; 32];

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub const PALLET_PROXY_ACCOUNT: [u8; 32] = [6u8; 32];
// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.

pub trait LiquidityModifier {
	type AssetId;
	type AccountId;
	fn on_deposit(account: Self::AccountId, asset: Self::AssetId, balance: u128) -> DispatchResult;
	fn on_withdraw(
		account: Self::AccountId,
		proxy_account: Self::AccountId,
		asset: Self::AssetId,
		balance: u128,
		do_force_withdraw: bool,
	) -> DispatchResult;
	fn on_register(main_account: Self::AccountId, proxy: Self::AccountId) -> DispatchResult;
}

#[frame_support::pallet]
pub mod pallet {
	use core::fmt::Debug;
	// Import various types used to declare pallet in scope.
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		traits::{Currency, ReservableCurrency},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use polkadex_primitives::AssetId;
	use sp_runtime::{
		traits::{AccountIdConversion, IdentifyAccount, Verify},
		SaturatedConversion,
	};
	/// Our pallet's configuration trait. All our types and constants go in here. If the
	/// pallet is dependent on specific other pallets, then their configuration traits
	/// should be added to our implied traits list.
	///
	/// `frame_system::Config` should always be included.
	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Address which holds the customer funds.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Balances Pallet
		type NativeCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

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

		/// Governance Origin
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		type CallOcex: LiquidityModifier<AssetId = AssetId, AccountId = Self::AccountId>;
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		/// Pallet already register
		PalletAlreadyRegistered,
		/// Unable to create proxy account
		UnableToCreateProxyAccount,
		/// Account not register
		PalletAccountNotRegistered,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register pallet account into orderbook
		///
		/// # Parameters
		///
		/// * `origin`: governance
		#[pallet::weight(10_000)]
		pub fn register_account(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let main_account = Self::get_pallet_account();
			let proxy_account = T::AccountId::decode(&mut &PROXY_ACCOUNT_BYTES[..])
				.map_err(|_| Error::<T>::UnableToCreateProxyAccount)?;
			ensure!(!<PalletRegister<T>>::get(), Error::<T>::PalletAlreadyRegistered);
			T::CallOcex::on_register(main_account.clone(), proxy_account.clone())?;
			<PalletRegister<T>>::put(true);
			Self::deposit_event(Event::PalletAccountRegister { main_account, proxy_account });
			Ok(())
		}

		/// Deposit assets to orderbook
		///
		/// # Parameters
		///
		/// * `origin`: governance
		/// * `asset`: asset id to deposit
		/// * `amount`: amount to deposit
		#[pallet::weight(10_000)]
		pub fn deposit_to_orderbook(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(<PalletRegister<T>>::get(), Error::<T>::PalletAccountNotRegistered);
			T::CallOcex::on_deposit(Self::get_pallet_account(), asset, amount.saturated_into())?;
			Ok(())
		}

		/// Withdraw assets from orderbook
		///
		/// # Parameters
		///
		/// * `origin`: governance
		/// * `asset`: asset id to withdraw
		/// * `amount`: amount to withdraw
		/// * `do_force_withdraw`: if set to true all active orders will be canceled and then the
		///   given amount will be withdrawn
		#[pallet::weight(10_000)]
		pub fn withdraw_from_orderbook(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
			do_force_withdraw: bool,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(<PalletRegister<T>>::get(), Error::<T>::PalletAccountNotRegistered);
			let proxy_account = T::AccountId::decode(&mut &PROXY_ACCOUNT_BYTES[..])
				.map_err(|_| Error::<T>::UnableToCreateProxyAccount)?;

			T::CallOcex::on_withdraw(
				Self::get_pallet_account(),
				proxy_account,
				asset,
				amount.saturated_into(),
				do_force_withdraw,
			)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_pallet_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn is_pallet_register)]
	pub(super) type PalletRegister<T: Config> = StorageValue<_, bool, ValueQuery>;
	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PalletAccountRegister { main_account: T::AccountId, proxy_account: T::AccountId },
	}
}
