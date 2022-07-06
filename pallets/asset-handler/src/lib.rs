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

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::AssetHandlerWeightInfo;
	use chainbridge::{BridgeChainId, ResourceId};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::fungibles::{Create, Inspect, Mutate},
			Currency, ExistenceRequirement, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{H160, U256};
	use sp_core::crypto::AccountId32;
	use sp_runtime::{
		traits::{One, UniqueSaturatedInto},
		SaturatedConversion,
	};

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config: frame_system::Config + chainbridge::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Asset Manager
		type AssetManager: Create<<Self as frame_system::Config>::AccountId>
			+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
			+ Inspect<<Self as frame_system::Config>::AccountId>;

		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		/// Treasury PalletId
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		type WeightInfo: AssetHandlerWeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// List of relayers who can relay data from Ethereum
	#[pallet::storage]
	#[pallet::getter(fn get_bridge_fee)]
	pub(super) type BridgeFee<T: Config> =
		StorageMap<_, Blake2_128Concat, BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Registered
		AssetRegistered(ResourceId),
		/// Asset Deposited (Recipient, ResourceId, Amount)
		AssetDeposited(T::AccountId, ResourceId, BalanceOf<T>),
		/// Asset Withdrawn (Recipient, ResourceId, Amount)
		AssetWithdrawn(H160, ResourceId, BalanceOf<T>),
		FeeUpdated(BridgeChainId, BalanceOf<T>),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Migration is not operational yet
		NotOperational,
		/// MinterMustBeRelayer
		MinterMustBeRelayer,
		/// ChainIsNotWhitelisted
		ChainIsNotWhitelisted,
		/// NotEnoughBalance
		NotEnoughBalance,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates new Asset where AssetId is derived from chain_id and contract Address
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `chain_id`: Asset's native chain
		/// * `contract_add`: Asset's actual address at native chain
		#[pallet::weight(T::WeightInfo::create_asset(1))]
		pub fn create_asset(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			contract_add: H160,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let rid = chainbridge::derive_resource_id(chain_id, &contract_add.0);
			T::AssetManager::create(
				Self::convert_asset_id(rid),
				chainbridge::Pallet::<T>::account_id(),
				true,
				BalanceOf::<T>::one().unique_saturated_into(),
			)?;
			Self::deposit_event(Event::<T>::AssetRegistered(rid));
			Ok(())
		}

		/// Mints Asset into Recipient's Account
		/// Only Relayers can call it.
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `destination_add`: Recipient's Account
		/// * `amount`: Amount to be minted in Recipient's Account
		/// * `rid`: Resource ID
		#[pallet::weight(T::WeightInfo::mint_asset(1))]
		pub fn mint_asset(
			origin: OriginFor<T>,
			destination_acc: T::AccountId,
			amount: BalanceOf<T>,
			rid: ResourceId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(chainbridge::Pallet::<T>::account_id() == sender, Error::<T>::MinterMustBeRelayer);
			T::AssetManager::mint_into(
				Self::convert_asset_id(rid),
				&destination_acc,
				amount.saturated_into::<u128>(),
			)?;
			Self::deposit_event(Event::<T>::AssetDeposited(destination_acc, rid, amount));
			Ok(())
		}

		/// Transfers Asset to Destination Chain.
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `chain_id`: Asset's native chain
		/// * `contract_add`: Asset's actual address at native chain
		/// * `amount`: Amount to be burned and transferred from Sender's Account
		/// * `recipient`: recipient
		#[pallet::weight(T::WeightInfo::withdraw(1, 1))]
		pub fn withdraw(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			contract_add: H160,
			amount: BalanceOf<T>,
			recipient: H160,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				chainbridge::Pallet::<T>::chain_whitelisted(chain_id),
				Error::<T>::ChainIsNotWhitelisted
			);
			let rid = chainbridge::derive_resource_id(chain_id, &contract_add.0);
			ensure!(
				T::AssetManager::reducible_balance(Self::convert_asset_id(rid), &sender, true) >=
					amount.saturated_into::<u128>(),
				Error::<T>::NotEnoughBalance
			);
			let fee = Self::fee_calculation(chain_id, amount);
			T::Currency::transfer(
				&sender,
				&chainbridge::Pallet::<T>::account_id(),
				fee,
				ExistenceRequirement::KeepAlive,
			)?;
			T::AssetManager::burn_from(
				Self::convert_asset_id(rid),
				&sender,
				amount.saturated_into::<u128>(),
			)?;
			chainbridge::Pallet::<T>::transfer_fungible(
				chain_id,
				rid,
				recipient.0.to_vec(),
				Self::convert_balance_to_eth_type(amount),
			)?;
			Self::deposit_event(Event::<T>::AssetWithdrawn(contract_add, rid, amount));
			Ok(())
		}

		/// Updates fee for given Chain id.
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `chain_id`: Asset's native chain
		/// * `min_fee`: Minimum fee to be charged to transfer Asset to different.
		/// * `fee_scale`: Scale to find fee depending on amount.
		#[pallet::weight(T::WeightInfo::update_fee(1, 1))]
		pub fn update_fee(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			min_fee: BalanceOf<T>,
			fee_scale: u32,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<BridgeFee<T>>::insert(chain_id, (min_fee, fee_scale));
			Self::deposit_event(Event::<T>::FeeUpdated(chain_id, min_fee));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn convert_balance_to_eth_type(balance: BalanceOf<T>) -> U256 {
			let balance: u128 = balance.unique_saturated_into();
			U256::from(balance).saturating_mul(U256::from(1000000u128))
		}

		fn fee_calculation(bridge_id: BridgeChainId, amount: BalanceOf<T>) -> BalanceOf<T> {
			let (min_fee, fee_scale) = Self::get_bridge_fee(bridge_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			}
		}

		pub fn convert_asset_id(token: ResourceId) -> u128 {
			let mut temp = [0u8; 16];
			temp.copy_from_slice(&token[0..16]);
			//temp.copy_fro	m_slice(token.as_fixed_bytes().as_ref());
			u128::from_le_bytes(temp)
		}

		#[cfg(feature = "runtime-benchmarks")]
		pub fn register_asset(rid: ResourceId) {
			T::AssetManager::create(
				Self::convert_asset_id(rid),
				chainbridge::Pallet::<T>::account_id(),
				true,
				BalanceOf::<T>::one().unique_saturated_into(),
			)
			.expect("Asset not Registered");
		}

		#[cfg(feature = "runtime-benchmarks")]
		pub fn mint_token(account: T::AccountId, rid: ResourceId, amount: u128) {
			T::AssetManager::mint_into(Pallet::<T>::convert_asset_id(rid), &account, amount);
		}
	}
}
