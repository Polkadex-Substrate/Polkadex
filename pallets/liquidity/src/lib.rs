// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]

use frame_support::{dispatch::DispatchResult, pallet_prelude::Weight, traits::Currency};
use pallet_timestamp as timestamp;
use sp_std::prelude::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const LENGTH_OF_HALF_BYTES: usize = 16;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.

// Trait to add liquidity in OCEX pallet
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
	#[cfg(feature = "runtime-benchmarks")]
	fn set_exchange_state_to_true() -> DispatchResult;
	#[cfg(feature = "runtime-benchmarks")]
	fn allowlist_and_create_token(account: Self::AccountId, token: u128) -> DispatchResult;
}

pub trait WeightInfo {
	fn register_account(_a: u32) -> Weight;
	fn deposit_to_orderbook(_a: u32, _i: u32, _z: u32) -> Weight;
	fn withdraw_from_orderbook(_a: u32, _i: u32, _z: u32) -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
	use core::fmt::Debug;
	// use thea_primitives::liquidity::LiquidityModifier;
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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		type CallOcex: LiquidityModifier<AssetId = AssetId, AccountId = Self::AccountId>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
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
		/// Unable to create proxy account
		UnableToCreateMainAccount,
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
		/// * `account_generation_key`: u32 value that will be used to generate main account and
		///   proxy account
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(0)]
		pub fn register_account(
			origin: OriginFor<T>,
			account_generation_key: u32,
		) -> DispatchResult {
			//ensure called by governance
			T::GovernanceOrigin::ensure_origin(origin)?;

			//ensure account not register already
			ensure!(
				!<RegisterGovernanceAccounts<T>>::contains_key(account_generation_key),
				Error::<T>::PalletAlreadyRegistered
			);

			//create main account and proxy account
			let main_account = Self::generate_main_account(account_generation_key)?;

			let proxy_account = Self::generate_proxy_account(account_generation_key)?;

			//call ocex register
			T::CallOcex::on_register(main_account.clone(), proxy_account.clone())?;

			//insert accounts in storage
			<RegisterGovernanceAccounts<T>>::insert(
				account_generation_key,
				(main_account.clone(), proxy_account.clone()),
			);
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
		/// * `account_generation_key`: u32 value that was used to generate main account and proxy
		///   account
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(1)]
		pub fn deposit_to_orderbook(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
			account_generation_key: u32,
		) -> DispatchResult {
			//ensure called by governance
			T::GovernanceOrigin::ensure_origin(origin)?;

			//check if the account present
			let (main_account, _) =
				<RegisterGovernanceAccounts<T>>::try_get(account_generation_key)
					.map_err(|_| Error::<T>::PalletAccountNotRegistered)?;

			//call ocex deposit
			T::CallOcex::on_deposit(main_account.clone(), asset, amount.saturated_into())?;

			Self::deposit_event(Event::DepositToPalletAccount { main_account, asset, amount });

			Ok(())
		}

		/// Withdraw assets from orderbook
		///
		/// # Parameters
		///
		/// * `origin`: governance
		/// * `asset`: asset id to withdraw
		/// * `amount`: amount to withdraw
		/// * `do_force_withdraw`: if set to true all active orders will be canceled from orderbook
		/// * `account_generation_key`: u32 value that was used to generate main account and proxy
		///  account given amount will be withdrawn
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(2)]
		pub fn withdraw_from_orderbook(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
			do_force_withdraw: bool,
			account_generation_key: u32,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			//check if the account present
			let (main_account, proxy_account) =
				<RegisterGovernanceAccounts<T>>::try_get(account_generation_key)
					.map_err(|_| Error::<T>::PalletAccountNotRegistered)?;

			//call ocex withdraw
			T::CallOcex::on_withdraw(
				main_account.clone(),
				proxy_account,
				asset,
				amount.saturated_into(),
				do_force_withdraw,
			)?;

			Self::deposit_event(Event::WithdrawFromPalletAccount { main_account, asset, amount });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn get_pallet_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		// To generate proxy account value provided by governance is used.
		pub fn generate_proxy_account(
			value_provided_by_governance: u32,
		) -> Result<T::AccountId, Error<T>> {
			let mut result = [0u8; 32];
			let mut last_index = 0;

			for _ in 0..8 {
				value_provided_by_governance
					.to_le_bytes()
					.into_iter()
					.enumerate()
					.for_each(|v| result[v.0 + last_index] = v.1);
				last_index += 4;
			}

			let proxy_account = T::AccountId::decode(&mut &result[..])
				.map_err(|_| Error::<T>::UnableToCreateProxyAccount)?;
			Ok(proxy_account)
		}

		// To generate main account initial half bytes are used from pallet account while rest from
		// value provided by governance.
		pub fn generate_main_account(
			value_provided_by_governance: u32,
		) -> Result<T::AccountId, Error<T>> {
			let mut result = [0u8; 32];
			let mut last_index = 0;
			let decoded_pallet_account_to_value =
				T::AccountId::encoded_size(&Self::get_pallet_account()) as u32;

			for _ in 0..8 {
				if last_index < LENGTH_OF_HALF_BYTES {
					decoded_pallet_account_to_value
						.to_le_bytes()
						.into_iter()
						.enumerate()
						.for_each(|v| result[v.0 + last_index] = v.1);
				} else {
					value_provided_by_governance
						.to_le_bytes()
						.into_iter()
						.enumerate()
						.for_each(|v| result[v.0 + last_index] = v.1);
				}
				last_index += 4;
			}

			let main_account = T::AccountId::decode(&mut &result[..])
				.map_err(|_| Error::<T>::UnableToCreateMainAccount)?;
			Ok(main_account)
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn is_account_register)]
	pub(super) type RegisterGovernanceAccounts<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, (T::AccountId, T::AccountId), OptionQuery>;
	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		PalletAccountRegister {
			main_account: T::AccountId,
			proxy_account: T::AccountId,
		},
		DepositToPalletAccount {
			main_account: T::AccountId,
			asset: AssetId,
			amount: BalanceOf<T>,
		},
		WithdrawFromPalletAccount {
			main_account: T::AccountId,
			asset: AssetId,
			amount: BalanceOf<T>,
		},
	}
}
