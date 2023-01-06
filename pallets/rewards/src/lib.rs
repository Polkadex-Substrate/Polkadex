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
	traits::{fungibles::Mutate, Currency, ExistenceRequirement, LockIdentifier},
	BoundedVec,
};
use polkadex_primitives::assets::AssetId;

use pallet_timestamp::{self as timestamp};
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
use sp_std::prelude::*;

use sp_runtime::SaturatedConversion;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod mock;

const UNIT_BALANCE: u128 = 1000000000000;
const MIN_REWARDS_CLAIMABLE_AMOUNT: u128 = 1000000000000;
pub const REWARDS_LOCK_ID: LockIdentifier = *b"REWARDID";

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
			Currency, LockableCurrency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;

	use sp_runtime::traits::{IdentifyAccount, Verify};

	use frame_support::traits::WithdrawReasons;
	use sp_std::cmp::min;
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
		type NativeCurrency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

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
		/// reward_id: The reward id
		#[pallet::weight(10_000)]
		pub fn create_reward_cycle(
			origin: OriginFor<T>,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
			intial_percentage: u32,
			reward_id: u32,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin.clone())?;

			//check to ensure no dupicate id gets added
			ensure!(!<IntializeRewards<T>>::contains_key(reward_id), Error::<T>::DuplicateId);

			//check to ensure start block greater than end block
			ensure!(start_block < end_block, Error::<T>::InvalidParameter);

			ensure!(
				intial_percentage <= 100 && intial_percentage > 0,
				Error::<T>::InvalidParameter
			);

			let reward_info = RewardInfo { start_block, end_block, intial_percentage };

			//inserting rewards info into storage
			<IntializeRewards<T>>::insert(reward_id, reward_info);

			Self::deposit_event(Event::RewardCycleCreated { start_block, end_block, reward_id });

			Ok(())
		}

		///The extrinsic will add beneficiaries for particular reward id
		/// Parameters,
		/// origin: The donor for the particular reward id
		/// id: Reward id
		/// beneficiaries: The accountid who can claim the reward
		/// u128: the value provide here considers 1 unit = 10^12
		#[pallet::weight(10_000)]
		pub fn add_reward_beneficiaries(
			origin: OriginFor<T>,
			reward_id: u32,
			conversion_factor: u128,
			beneficiaries: BoundedVec<
				(T::AccountId, BalanceOf<T>),
				polkadex_primitives::ingress::HandleBalanceLimit,
			>,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin.clone())?;

			//check if reward id present in storage
			ensure!(
				<IntializeRewards<T>>::contains_key(&reward_id),
				Error::<T>::RewardIdNotRegister
			);

			if let Some(reward_info) = <IntializeRewards<T>>::get(reward_id) {
				//add all the beneficiary account in storage
				for beneficiary in beneficiaries {
					//generate lock id

					//calculate total rewards receive based on the factor
					let contribution: u128 = beneficiary.1.saturated_into();
					let total_rewards_in_pdex: BalanceOf<T> = contribution
						.saturating_mul(conversion_factor)
						.saturating_div(UNIT_BALANCE)
						.saturated_into();
					let reward_info = RewardInfoForAccount {
						total_reward_amount: total_rewards_in_pdex,
						claim_amount: 0_u128.saturated_into(),
						is_intial_rewards_claimed: false,
						is_intialized: false,
						lock_id: REWARDS_LOCK_ID,
						last_block_rewards_claim: reward_info.start_block,
					};
					<Distributor<T>>::insert(reward_id, beneficiary.0, reward_info);
				}
			} else {
				//will not occur since we are already ensuring it above, still sanity check
				return Err(Error::<T>::RewardIdNotRegister.into());
			}

			Ok(())
		}

		///The extrinsic will unlock users reward
		/// Parameters,
		/// origin: The users address which has been mapped to reward id
		/// id: Reward id
		#[pallet::weight(10_000)]
		pub fn unlock_reward(origin: OriginFor<T>, reward_id: u32) -> DispatchResult {
			let user: T::AccountId = ensure_signed(origin)?;

			//check if given id valid or not
			ensure!(
				<IntializeRewards<T>>::contains_key(reward_id),
				Error::<T>::RewardIdNotRegister
			);

			//check if user is added in reward list
			ensure!(<Distributor<T>>::contains_key(reward_id, &user), Error::<T>::UserNotEligible);

			//transfer funds to users account and lock it
			ensure!(
				<Distributor<T>>::mutate(reward_id, user.clone(), |user_reward_info| {
					if let Some(user_reward_info) = user_reward_info {
						//transfer funds from pallet account to users account
						ensure!(
							Self::transfer_pdex_rewards(
								&Self::get_pallet_account(),
								&user,
								user_reward_info.total_reward_amount
							)
							.is_ok(),
							Error::<T>::TransferFailed
						);

						//lock funds in users account
						T::NativeCurrency::set_lock(
							REWARDS_LOCK_ID,
							&user,
							user_reward_info.total_reward_amount.saturated_into(),
							WithdrawReasons::TRANSFER,
						);

						user_reward_info.is_intialized = true;
						Ok(())
					} else {
						//sanity check
						Err(Error::<T>::UserNotEligible)
					}
				})
				.is_ok(),
				Error::<T>::TransferFailed
			);

			Self::deposit_event(Event::UserUnlockedReward { user, reward_id });
			Ok(())
		}

		/// The user will use the extrinsic to claim rewards
		/// origin: The users address which has been mapped to reward id
		/// id: The reward id
		#[pallet::weight(10_000)]
		pub fn claim(origin: OriginFor<T>, reward_id: u32) -> DispatchResult {
			//ToDo can only be called if user has called intialize

			let user: T::AccountId = ensure_signed(origin)?;

			//check if given id valid or not
			ensure!(
				<IntializeRewards<T>>::contains_key(reward_id),
				Error::<T>::RewardIdNotRegister
			);

			//check if user is added in reward list
			ensure!(<Distributor<T>>::contains_key(reward_id, &user), Error::<T>::UserNotEligible);

			ensure!(
				<Distributor<T>>::mutate(reward_id, user.clone(), |user_reward_info| {
					if let Some(reward_info) = <IntializeRewards<T>>::get(&reward_id) {
						if let Some(user_reward_info) = user_reward_info {
							//check if user has intialize rewards or not
							ensure!(
								user_reward_info.is_intialized,
								Error::<T>::UserHasNotIntializeClaimRewards
							);

							let mut rewards_claimable: u128 = 0_u128.saturated_into();

							//calculate the intial rewards that can be claimed
							let intial_rewards_claimed = user_reward_info
								.total_reward_amount
								.saturated_into::<u128>()
								.saturating_mul(reward_info.intial_percentage as u128)
								.saturating_div(100);

							//if intial rewards are not claimed add it to claimable rewards
							if user_reward_info.is_intial_rewards_claimed == false {
								rewards_claimable = intial_rewards_claimed;
							}

							//calculate the number of blocks the user can claim rewards
							let current_block_no: u128 =
								<frame_system::Pallet<T>>::block_number().saturated_into();
							let last_reward_claimed_block_no: u128 =
								user_reward_info.last_block_rewards_claim.saturated_into();
							let unclaimed_blocks: u128 = min(
								current_block_no,
								reward_info.end_block.saturated_into::<u128>(),
							)
							.saturating_sub(last_reward_claimed_block_no);

							let crowdloan_period = reward_info
								.end_block
								.saturated_into::<u128>()
								.saturating_sub(reward_info.start_block.saturated_into::<u128>());

							//calculate custom factor for the user
							// Formula = (total_rewards - intial_rewards_claimed) / crowloan_period
							let factor = user_reward_info
								.total_reward_amount
								.saturated_into::<u128>()
								.saturating_sub(intial_rewards_claimed)
								.saturating_div(crowdloan_period);

							// add the unclaimed block rewards to claimable rewards
							rewards_claimable = rewards_claimable
								.saturating_add(factor.saturating_mul(unclaimed_blocks));

							//ensure the claimable amount is greater than min claimable amount
							ensure!(
								rewards_claimable > MIN_REWARDS_CLAIMABLE_AMOUNT,
								Error::<T>::AmountToLowtoRedeem
							);

							//remove lock
							T::NativeCurrency::remove_lock(user_reward_info.lock_id, &user);

							//update storage
							user_reward_info.last_block_rewards_claim =
								<frame_system::Pallet<T>>::block_number();
							user_reward_info.is_intial_rewards_claimed = true;
							user_reward_info.claim_amount = user_reward_info
								.claim_amount
								.saturated_into::<u128>()
								.saturating_add(rewards_claimable)
								.saturated_into();

							//set new lock
							let reward_amount_to_lock = user_reward_info
								.total_reward_amount
								.saturated_into::<u128>()
								.saturating_sub(
									user_reward_info.claim_amount.saturated_into::<u128>(),
								);
							T::NativeCurrency::set_lock(
								user_reward_info.lock_id,
								&user,
								reward_amount_to_lock.saturated_into(),
								WithdrawReasons::TRANSFER,
							);

							Self::deposit_event(Event::UserClaimedReward {
								user,
								reward_id,
								claimed: rewards_claimable.saturated_into(),
							});

							Ok(())
						} else {
							//will not occur since we are already ensuring it above, sanity check
							return Err(Error::<T>::UserNotEligible);
						}
					} else {
						// will not occur since we are already ensuring it above, sanity check
						return Err(Error::<T>::RewardIdNotRegister);
					}
				})
				.is_ok(),
				Error::<T>::TransferFailed
			);

			Ok(())
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		RewardCycleCreated {
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
			reward_id: u32,
		},
		UserUnlockedReward {
			user: T::AccountId,
			reward_id: u32,
		},
		UserClaimedReward {
			user: T::AccountId,
			reward_id: u32,
			claimed: BalanceOf<T>,
		},
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
		/// User not eligible for the reward
		UserNotEligible,
		/// Transfer of funds failed
		TransferFailed,
		/// Amount to low to reedeem
		AmountToLowtoRedeem,
		/// User needs to intialize first before claiming rewards
		UserHasNotIntializeClaimRewards,
	}

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	#[scale_info(bounds(), skip_type_params(T))]
	pub struct RewardInfo<T: Config> {
		pub start_block: T::BlockNumber,
		pub end_block: T::BlockNumber,
		pub intial_percentage: u32, //todo: u32 value just taken for testing purpose needs to ba change
	}

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	#[scale_info(bounds(), skip_type_params(T))]
	pub struct RewardInfoForAccount<T: Config> {
		pub total_reward_amount: BalanceOf<T>,
		pub claim_amount: BalanceOf<T>,
		pub is_intial_rewards_claimed: bool,
		pub is_intialized: bool,
		pub lock_id: [u8; 8],
		pub last_block_rewards_claim: T::BlockNumber,
	}

	///Allowlisted tokens
	#[pallet::storage]
	#[pallet::getter(fn get_id)]
	pub(super) type AllowlistedToken<T: Config> = StorageValue<_, u8, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_beneficary)]
	pub(super) type IntializeRewards<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, RewardInfo<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_account_reward_info)]
	pub(super) type Distributor<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u32,
		Blake2_128Concat,
		T::AccountId,
		RewardInfoForAccount<T>,
		OptionQuery,
	>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	fn get_pallet_account() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	//The followling function will be used by claim extrinsic to tranfer balance from donor to beneficiary

	fn transfer_pdex_rewards(
		payer: &T::AccountId,
		payee: &T::AccountId,
		amount: BalanceOf<T>,
	) -> DispatchResult {
		T::NativeCurrency::transfer(
			payer,
			payee,
			amount.unique_saturated_into(),
			ExistenceRequirement::KeepAlive,
		)?;
		Ok(())
	}
}
