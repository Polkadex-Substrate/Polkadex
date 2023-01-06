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
use polkadex_primitives::assets::AssetId;

use pallet_timestamp::{self as timestamp};
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
use sp_std::prelude::*;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
	use core::fmt::Debug;
	// Import various types used to declare pallet in scope.
	use super::*;
	use frame_support::{
		pallet_prelude::{OptionQuery, *},
		traits::{
			fungibles::{Create, Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use polkadex_primitives::Balance as RewardBalance;
	use sp_runtime::traits::{IdentifyAccount, Verify};
	use sp_std::convert::TryInto;

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

		/// Governance Origin
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		//ToDo: on_intialize would be added to clear events, to clear storage if everyone has been donated
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The extrinsic will be used to start a new reward cycle
		/// Parameters
		/// origin: The donor who wants to start the reward cycle
		/// start_block: The block from which reward distribution will start
		/// end_block: The block at which last rewards will be distributed
		/// intial_percentage: The percentage of rewards that can be claimed at start block
		/// id: The reward id
		#[pallet::weight(10_000)]
		pub fn create_reward_cycle(
			origin: OriginFor<T>,
			start_block: u32,
			end_block: u32,
			intial_percentage: u32,
			id: u32,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin.clone())?;

			//check to ensure no dupicate id gets added
			ensure!(!<IntializeRewards<T>>::contains_key(id), Error::<T>::DuplicateId);

			//check to ensure start block greater than end block
			ensure!(start_block < end_block, Error::<T>::InvalidParameter);

			let reward_info = RewardInfo { start_block, end_block, intial_percentage };

			//inserting rewards info into storage
			<IntializeRewards<T>>::insert(id, reward_info);

			Self::deposit_event(Event::RewardCycleCreated { start_block, end_block, id });

			Ok(())
		}

		///The extrinsic will add beneficiaries for particular reward id
		/// Parameters,
		/// origin: The donor for the particular reward id
		/// id: Reward id
		/// beneficiaries: The accountid who can claim the reward
		#[pallet::weight(10_000)]
		pub fn add_reward_beneficiaries(
			origin: OriginFor<T>,
			id: u32,
			beneficiaries: BoundedVec<
				(T::AccountId, u128),
				polkadex_primitives::ingress::HandleBalanceLimit,
			>,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin.clone())?;

			//check if reward id present in storage
			ensure!(<IntializeRewards<T>>::contains_key(&id), Error::<T>::RewardIdNotRegister);

			//add all the beneficiary account in storage
			for beneficiary in beneficiaries {
				let reward_info = RewardInfoForAccount {
					total_amount: beneficiary.1,
					claim_amount: 0,
					staked_amount: 0,
					last_block_rewards_claim: 0,
				};
				<Distributor<T>>::insert(id, beneficiary.0, reward_info);
			}

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn claim(_origin: OriginFor<T>) -> DispatchResult {
			//ToDo: issue no -- will add the followling extrinsic
			Ok(())
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RewardCycleCreated { start_block: u32, end_block: u32, id: u32 },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The id has already been taken
		DuplicateId,
		/// start block should be smaller than end block
		InvalidParameter,
		/// reward id doesn't correctly map to donor
		IncorrectDonorAccount,
		/// The reward Id is not register
		RewardIdNotRegister,
	}

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	pub struct RewardInfo {
		pub start_block: u32,
		pub end_block: u32,
		pub intial_percentage: u32, //todo: u32 value just taken for testing purpose needs to ba change
	}

	type RewardInfoOf = RewardInfo;

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	pub struct RewardInfoForAccount {
		pub total_amount: RewardBalance,
		pub claim_amount: RewardBalance,
		pub staked_amount: RewardBalance,
		pub last_block_rewards_claim: u32,
	}

	type RewardInfoForAccountIs = RewardInfoForAccount;

	#[pallet::storage]
	#[pallet::getter(fn get_beneficary)]
	pub(super) type IntializeRewards<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, RewardInfoOf, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_account_reward_info)]
	pub(super) type Distributor<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Blake2_128Concat,
		T::AccountId,
		RewardInfoForAccountIs,
		OptionQuery,
	>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	fn _get_pallet_account() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	//The followling function will be used by claim extrinsic to tranfer balance from donor to beneficiary
	fn _transfer_asset(
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
