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
	traits::{Currency, ExistenceRequirement, LockIdentifier},
	BoundedVec,
};
use pallet_timestamp::{self as timestamp};
use sp_runtime::{
	traits::{AccountIdConversion, UniqueSaturatedInto},
	SaturatedConversion,
};
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

mod crowloan_rewardees;

const UNIT_BALANCE: u128 = 1_000_000_000_000;
const MIN_REWARDS_CLAIMABLE_AMOUNT: u128 = UNIT_BALANCE;
pub const REWARDS_LOCK_ID: LockIdentifier = *b"REWARDID";
pub const MIN_DIFFERENCE_BETWEEN_START_AND_END_BLOCK: u128 = 15;

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
		traits::{Currency, LockableCurrency, ReservableCurrency, WithdrawReasons},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use polkadex_primitives::UNIT_BALANCE;
	use sp_runtime::traits::{IdentifyAccount, Verify};
	use sp_std::{cmp::min, convert::TryInto};
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

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The extrinsic will be used to start a new reward cycle
		/// # Parameters
		/// * `origin`: The donor who wants to start the reward cycle
		/// * `start_block`: The block from which reward distribution will start
		/// * `end_block`: The block at which last rewards will be distributed
		/// * `initial_percentage`: The percentage of rewards that can be claimed at start block
		/// * `reward_id`: The reward id
		#[pallet::weight(10_000)]
		pub fn create_reward_cycle(
			origin: OriginFor<T>,
			start_block: T::BlockNumber,
			end_block: T::BlockNumber,
			initial_percentage: u32,
			reward_id: u32,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin.clone())?;

			//check to ensure no duplicate id gets added
			ensure!(!<InitializeRewards<T>>::contains_key(reward_id), Error::<T>::DuplicateId);

			//check to ensure start block greater than end block
			ensure!(start_block < end_block, Error::<T>::InvalidBlocksRange);

			//ensure that difference between start of vesting period - current block is greater
			// than min difference
			let difference_between_start_and_current_block = start_block
				.saturated_into::<u128>()
				.saturating_sub(<frame_system::Pallet<T>>::block_number().saturated_into::<u128>());
			ensure!(
				difference_between_start_and_current_block >
					MIN_DIFFERENCE_BETWEEN_START_AND_END_BLOCK,
				Error::<T>::InvalidBlocksRange
			);

			//ensure percentage range is valid
			ensure!(
				initial_percentage <= 100 && initial_percentage > 0,
				Error::<T>::InvalidInitialPercentage
			);

			let reward_info = RewardInfo { start_block, end_block, initial_percentage };

			//inserting reward info into the storage
			<InitializeRewards<T>>::insert(reward_id, reward_info);

			Self::deposit_event(Event::RewardCycleCreated { start_block, end_block, reward_id });

			Ok(())
		}

		///The extrinsic will add beneficiaries for particular reward id
		/// # Parameters,
		/// * `origin`: The donor for the particular reward id
		/// * `id`: Reward id
		/// * `conversion_factor`: The conversion factor from dot to pdex
		/// * `beneficiaries: The account id who can claim the reward & the amount in dot
		///   contributed
		/// base 10^12 u128: the value provide here considers 1 unit = 10^12
		#[pallet::weight(10_000)]
		pub fn add_reward_beneficiaries(
			origin: OriginFor<T>,
			reward_id: u32,
			conversion_factor: BalanceOf<T>,
			beneficiaries: BoundedVec<
				(T::AccountId, BalanceOf<T>),
				polkadex_primitives::ingress::HandleBalanceLimit,
			>,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin)?;

			//check if reward id present in storage
			ensure!(
				<InitializeRewards<T>>::contains_key(reward_id),
				Error::<T>::RewardIdNotRegister
			);

			if let Some(reward_info) = <InitializeRewards<T>>::get(reward_id) {
				//calculate crowdloan period
				let crowdloan_period = reward_info
					.end_block
					.saturated_into::<u128>()
					.saturating_sub(reward_info.start_block.saturated_into::<u128>());

				//add all the beneficiary account in storage
				for beneficiary in beneficiaries {
					//calculate total rewards receive based on the conversion factor
					let contribution: u128 = beneficiary.1.saturated_into();
					let total_rewards_in_pdex: BalanceOf<T> = contribution
						.saturating_mul(conversion_factor.saturated_into())
						.saturating_div(UNIT_BALANCE)
						.saturated_into();

					if total_rewards_in_pdex > MIN_REWARDS_CLAIMABLE_AMOUNT.saturated_into() {
						let initial_rewards_claimable: BalanceOf<T> = total_rewards_in_pdex
							.saturated_into::<u128>()
							.saturating_mul(reward_info.initial_percentage as u128)
							.saturating_div(100)
							.saturated_into();

						//calculate custom factor for the user
						// Formula = (total_rewards - initial_rewards_claimed) / crowdloan_period
						let factor: BalanceOf<T> = total_rewards_in_pdex
							.saturated_into::<u128>()
							.saturating_sub(initial_rewards_claimable.saturated_into::<u128>())
							.saturating_div(crowdloan_period)
							.saturated_into();

						let reward_info = RewardInfoForAccount {
							total_reward_amount: total_rewards_in_pdex,
							claim_amount: 0_u128.saturated_into(),
							is_initial_rewards_claimed: false,
							is_initialized: false,
							lock_id: REWARDS_LOCK_ID,
							last_block_rewards_claim: reward_info.start_block,
							initial_rewards_claimable,
							factor,
						};
						<Distributor<T>>::insert(reward_id, beneficiary.0, reward_info);
					} else {
						//emit a event
						Self::deposit_event(Event::UserRewardNotSatisfyingMinConstraint {
							user: beneficiary.0,
							amount_in_pdex: total_rewards_in_pdex,
							reward_id,
						});
					}
				}
			} else {
				//will not occur since we are already ensuring it above, still sanity check
				return Err(Error::<T>::RewardIdNotRegister.into())
			}
			Ok(())
		}

		///The extrinsic will transfer and lock users rewards in the users account
		/// # Parameters,
		/// * `origin`: The users address which has been mapped to reward id
		/// * `id`: Reward id
		#[pallet::weight(10_000)]
		pub fn initialize_claim_rewards(origin: OriginFor<T>, reward_id: u32) -> DispatchResult {
			let user: T::AccountId = ensure_signed(origin)?;
			//check if given id valid or not
			ensure!(
				<InitializeRewards<T>>::contains_key(reward_id),
				Error::<T>::RewardIdNotRegister
			);

			let account_in_vec: Vec<u8> = T::AccountId::encode(&user);

			//get reward info of user from const hash map
			if let Some((total_rewards_in_pdex, initial_rewards_claimable, factor)) =
				crowloan_rewardees::HASHMAP.get(&account_in_vec)
			{
				if let Some(reward_info) = <InitializeRewards<T>>::take(reward_id) {
					if total_rewards_in_pdex.clone() > MIN_REWARDS_CLAIMABLE_AMOUNT.saturated_into()
					{
						let reward_info = RewardInfoForAccount {
							total_reward_amount: (total_rewards_in_pdex.clone()).saturated_into(),
							claim_amount: 0_u128.saturated_into(),
							is_initial_rewards_claimed: false,
							is_initialized: false,
							lock_id: REWARDS_LOCK_ID,
							last_block_rewards_claim: reward_info.start_block,
							initial_rewards_claimable: (initial_rewards_claimable.clone())
								.saturated_into(),
							factor: (factor.clone()).saturated_into(),
						};
						<Distributor<T>>::insert(reward_id, user.clone(), reward_info);
					}
				} else {
					//sanity check
					return Err(Error::<T>::RewardIdNotRegister.into())
				}
			} else {
				return Err(Error::<T>::UserNotEligible.into())
			}

			//check if user is added in reward list
			ensure!(<Distributor<T>>::contains_key(reward_id, &user), Error::<T>::UserNotEligible);

			<Distributor<T>>::mutate(reward_id, user.clone(), |user_reward_info| {
				if let Some(user_reward_info) = user_reward_info {
					// only unlock reward if current block greater than or equal to the starting
					// block of reward
					if let Some(reward_info) = <InitializeRewards<T>>::get(reward_id) {
						ensure!(
							reward_info.start_block.saturated_into::<u128>() <=
								<frame_system::Pallet<T>>::block_number()
									.saturated_into::<u128>(),
							Error::<T>::RewardsCannotBeUnlockYet
						);
					}

					//check if user already unlocked the rewards
					ensure!(!user_reward_info.is_initialized, Error::<T>::RewardsAlreadyUnlocked);
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
					user_reward_info.is_initialized = true;
					Ok(())
				} else {
					//sanity check
					Err(Error::<T>::UserNotEligible)
				}
			})?;
			Self::deposit_event(Event::UserUnlockedReward { user, reward_id });
			Ok(())
		}

		/// The user will use the extrinsic to claim rewards
		/// # Parameters
		/// * `origin`: The users address which has been mapped to reward id
		/// * `id`: The reward id
		#[pallet::weight(10_000)]
		pub fn claim(origin: OriginFor<T>, reward_id: u32) -> DispatchResult {
			let user: T::AccountId = ensure_signed(origin)?;

			//check if given id valid or not
			ensure!(
				<InitializeRewards<T>>::contains_key(reward_id),
				Error::<T>::RewardIdNotRegister
			);

			//check if user is added in reward list
			ensure!(<Distributor<T>>::contains_key(reward_id, &user), Error::<T>::UserNotEligible);

			<Distributor<T>>::mutate(reward_id, user.clone(), |user_reward_info| {
				if let Some(reward_info) = <InitializeRewards<T>>::get(reward_id) {
					if let Some(user_reward_info) = user_reward_info {
						//check if user has initialize rewards or not
						ensure!(
							user_reward_info.is_initialized,
							Error::<T>::UserHasNotInitializeClaimRewards
						);

						let mut rewards_claimable: u128 = 0_u128.saturated_into();

						//if initial rewards are not claimed add it to claimable rewards
						if !user_reward_info.is_initial_rewards_claimed {
							rewards_claimable =
								user_reward_info.initial_rewards_claimable.saturated_into::<u128>();
						}

						//calculate the number of blocks the user can claim rewards
						let current_block_no: u128 =
							<frame_system::Pallet<T>>::block_number().saturated_into();
						let last_reward_claimed_block_no: u128 =
							user_reward_info.last_block_rewards_claim.saturated_into();
						let unclaimed_blocks: u128 =
							min(current_block_no, reward_info.end_block.saturated_into::<u128>())
								.saturating_sub(last_reward_claimed_block_no);

						// add the unclaimed block rewards to claimable rewards
						rewards_claimable = rewards_claimable.saturating_add(
							user_reward_info
								.factor
								.saturated_into::<u128>()
								.saturating_mul(unclaimed_blocks),
						);

						//ensure total_rewards_claimable - rewards_claimed >= rewards_claimable
						//sanity check
						ensure!(
							user_reward_info
								.total_reward_amount
								.saturated_into::<u128>()
								.saturating_sub(
									user_reward_info.claim_amount.saturated_into::<u128>()
								) >= rewards_claimable,
							Error::<T>::AllRewardsAlreadyClaimed
						);

						//ensure the claimable amount is greater than min claimable amount
						ensure!(
							rewards_claimable > MIN_REWARDS_CLAIMABLE_AMOUNT,
							Error::<T>::AmountToLowToRedeem
						);

						//remove lock
						T::NativeCurrency::remove_lock(user_reward_info.lock_id, &user);

						//update storage
						user_reward_info.last_block_rewards_claim =
							<frame_system::Pallet<T>>::block_number();
						user_reward_info.is_initial_rewards_claimed = true;
						user_reward_info.claim_amount = user_reward_info
							.claim_amount
							.saturated_into::<u128>()
							.saturating_add(rewards_claimable)
							.saturated_into();

						//set new lock on new amount
						let reward_amount_to_lock = user_reward_info
							.total_reward_amount
							.saturated_into::<u128>()
							.saturating_sub(user_reward_info.claim_amount.saturated_into::<u128>());
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
						Err(Error::<T>::UserNotEligible)
					}
				} else {
					// will not occur since we are already ensuring it above, sanity check
					Err(Error::<T>::RewardIdNotRegister)
				}
			})?;

			Ok(())
		}
	}

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
		UserRewardNotSatisfyingMinConstraint {
			user: T::AccountId,
			amount_in_pdex: BalanceOf<T>,
			reward_id: u32,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The id has already been taken
		DuplicateId,
		/// Invalid block range provided
		InvalidBlocksRange,
		/// Invalid percentage range
		InvalidInitialPercentage,
		/// reward id doesn't correctly map to donor
		IncorrectDonorAccount,
		/// The reward Id is not register
		RewardIdNotRegister,
		/// User not eligible for the reward
		UserNotEligible,
		/// Transfer of funds failed
		TransferFailed,
		/// Amount to low to redeem
		AmountToLowToRedeem,
		/// User needs to initialize first before claiming rewards
		UserHasNotInitializeClaimRewards,
		/// User has already unlocked the rewards
		RewardsAlreadyUnlocked,
		/// Reward cycle need to get started before unlocking rewards
		RewardsCannotBeUnlockYet,
		/// User has already claimed all the available amount
		AllRewardsAlreadyClaimed,
	}

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	#[scale_info(bounds(), skip_type_params(T))]
	pub struct RewardInfo<T: Config> {
		pub start_block: T::BlockNumber,
		pub end_block: T::BlockNumber,
		pub initial_percentage: u32,
	}

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	#[scale_info(bounds(), skip_type_params(T))]
	pub struct RewardInfoForAccount<T: Config> {
		pub total_reward_amount: BalanceOf<T>,
		pub claim_amount: BalanceOf<T>,
		pub is_initial_rewards_claimed: bool,
		pub is_initialized: bool,
		pub lock_id: [u8; 8],
		pub last_block_rewards_claim: T::BlockNumber,
		pub initial_rewards_claimable: BalanceOf<T>,
		pub factor: BalanceOf<T>,
	}

	#[pallet::storage]
	#[pallet::getter(fn get_beneficary)]
	pub(super) type InitializeRewards<T: Config> =
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

impl<T: Config> Pallet<T> {
	fn get_pallet_account() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	//The following function will be used by initialize_claim_rewards extrinsic to transfer balance
	// from pallet account to beneficiary account
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
