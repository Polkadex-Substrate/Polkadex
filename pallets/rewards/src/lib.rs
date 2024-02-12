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

// Start of lease period: 2022-06-06 07:47
// End of the lease period: 2024-04-08 07:47
// Polkadex block 1,815,527 has the closest timestamp when the lease period started.
// 96 weeks =  672 days = 58060800 seconds = 4838400 blocks
// Start block on PDEX solo chain: 1815527
// End block on PDEX solo chain: 6653927

//! # Rewards Pallet.
//!
//! This pallet will help to provide "parachain" rewards to the participants in crowdloan.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]

use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::{Get, Weight},
	traits::{Currency, ExistenceRequirement, LockIdentifier},
};
use pallet_timestamp as timestamp;
use sp_runtime::{
	traits::{AccountIdConversion, UniqueSaturatedInto},
	SaturatedConversion, Saturating,
};
use sp_std::{cmp::min, prelude::*};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

pub trait WeightInfo {
	fn create_reward_cycle(_b: u32, _i: u32, _r: u32) -> Weight;
	fn initialize_claim_rewards() -> Weight;
	fn claim() -> Weight;
}
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod crowdloan_rewardees;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

const MIN_REWARDS_CLAIMABLE_AMOUNT: u128 = polkadex_primitives::UNIT_BALANCE;
pub const REWARDS_LOCK_ID: LockIdentifier = *b"REWARDID";
// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
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
	use polkadex_primitives::AccountId;
	use sp_runtime::traits::{IdentifyAccount, Verify};

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
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// The extrinsic will be used to start a new reward cycle.
		///
		/// # Parameters
		///
		/// * `origin`: The donor who wants to start the reward cycle.
		/// * `start_block`: The block from which reward distribution will start.
		/// * `end_block`: The block at which last rewards will be distributed.
		/// * `initial_percentage`: The percentage of rewards that can be claimed at start block.
		/// * `reward_id`: The reward id.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::create_reward_cycle(1, 1, 1))]
		pub fn create_reward_cycle(
			origin: OriginFor<T>,
			start_block: BlockNumberFor<T>,
			end_block: BlockNumberFor<T>,
			initial_percentage: u32,
			reward_id: u32,
		) -> DispatchResult {
			//check to ensure governance
			T::GovernanceOrigin::ensure_origin(origin.clone())?;

			//check to ensure no duplicate id gets added
			ensure!(!<InitializeRewards<T>>::contains_key(reward_id), Error::<T>::DuplicateId);

			//check to ensure start block greater than end block
			ensure!(start_block < end_block, Error::<T>::InvalidBlocksRange);

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

		/// The extrinsic will transfer and lock users rewards into users account.
		///
		/// # Parameters
		///
		/// * `origin`: The users address which has been mapped to reward id.
		/// * `reward_id`: Reward id.
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::initialize_claim_rewards())]
		pub fn initialize_claim_rewards(origin: OriginFor<T>, reward_id: u32) -> DispatchResult {
			let user: T::AccountId = ensure_signed(origin)?;

			// check if rewards can be unlocked at current block
			if let Some(reward_info) = <InitializeRewards<T>>::get(reward_id) {
				ensure!(
					reward_info.start_block.saturated_into::<u128>()
						<= <frame_system::Pallet<T>>::block_number().saturated_into::<u128>(),
					Error::<T>::RewardsCannotBeUnlockYet
				);
			} else {
				//reward id not register yet
				return Err(Error::<T>::RewardIdNotRegister.into());
			}

			//check if user has already initialize the reward
			ensure!(
				!<Distributor<T>>::contains_key(reward_id, &user),
				Error::<T>::RewardsAlreadyInitialized
			);

			let account_in_vec: [u8; 32] = T::AccountId::encode(&user)
				.try_into()
				.map_err(|_| Error::<T>::IncorrectDonorAccount)?;
			#[allow(clippy::borrow_interior_mutable_const)]
			#[allow(clippy::declare_interior_mutable_const)]
			//get info of user from pre defined hash map and add it in storage
			if let Some((_, (total_rewards_in_pdex, initial_rewards_claimable, factor))) =
				crowdloan_rewardees::HASHMAP
					.iter()
					.find(|a| a.0 == AccountId::new(account_in_vec))
			{
				//get reward info
				if let Some(reward_info) = <InitializeRewards<T>>::get(reward_id) {
					if *total_rewards_in_pdex > MIN_REWARDS_CLAIMABLE_AMOUNT {
						//initialize reward info struct
						let mut reward_info = RewardInfoForAccount {
							total_reward_amount: (*total_rewards_in_pdex).saturated_into(),
							claim_amount: 0_u128.saturated_into(),
							is_initial_rewards_claimed: false,
							is_initialized: false,
							lock_id: REWARDS_LOCK_ID,
							last_block_rewards_claim: reward_info.start_block,
							initial_rewards_claimable: (*initial_rewards_claimable)
								.saturated_into(),
							factor: (*factor).saturated_into(),
						};

						//transfer funds from pallet account to users account
						Self::transfer_pdex_rewards(
							&Self::get_pallet_account(),
							&user,
							reward_info.total_reward_amount,
						)?;

						//lock users funds in his account
						T::NativeCurrency::set_lock(
							REWARDS_LOCK_ID,
							&user,
							reward_info.total_reward_amount,
							WithdrawReasons::TRANSFER,
						);

						//set initialize flag as true
						reward_info.is_initialized = true;

						//insert reward info into storage
						<Distributor<T>>::insert(reward_id, user.clone(), reward_info);
					} else {
						return Err(Error::<T>::AmountToLowtoInitializeRewards.into());
					}
				} else {
					//sanity check
					return Err(Error::<T>::RewardIdNotRegister.into());
				}
			} else {
				return Err(Error::<T>::UserNotEligible.into());
			}

			Self::deposit_event(Event::UserUnlockedReward { user, reward_id });
			Ok(())
		}

		/// The user will use the extrinsic to claim rewards.
		///
		/// # Parameters
		///
		/// * `origin`: The users address which has been mapped to reward id.
		/// * `id`: The reward id.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::claim())]
		pub fn claim(origin: OriginFor<T>, reward_id: u32) -> DispatchResult {
			let user: T::AccountId = ensure_signed(origin)?;
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
						//user not present in reward list
						Err(Error::<T>::UserNotEligible)
					}
				} else {
					// given reward id not valid
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
			start_block: BlockNumberFor<T>,
			end_block: BlockNumberFor<T>,
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
		/// Reward cycle need to get started before unlocking rewards
		RewardsCannotBeUnlockYet,
		/// User has already claimed all the available amount
		AllRewardsAlreadyClaimed,
		/// User has already initialize the rewards
		RewardsAlreadyInitialized,
		/// Amount to low to initialize the rewards
		AmountToLowtoInitializeRewards,
	}

	#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Default)]
	#[scale_info(bounds(), skip_type_params(T))]
	pub struct RewardInfo<T: Config> {
		pub start_block: BlockNumberFor<T>,
		pub end_block: BlockNumberFor<T>,
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
		pub last_block_rewards_claim: BlockNumberFor<T>,
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

	/// Retrieves the rewards information associated with a given account and reward ID.
	///
	/// # Parameters
	///
	/// * `account_id`: The account ID for which the rewards information is to be fetched.
	/// * `reward_id`: The specific reward ID to fetch the rewards information.
	///
	/// # Returns
	///
	/// A `RewardsInfoByAccount` structure containing the claimed, unclaimed, and claimable
	/// rewards associated with the account and reward ID.
	pub fn account_info(
		account_id: T::AccountId,
		reward_id: u32,
	) -> Result<polkadex_primitives::rewards::RewardsInfoByAccount<u128>, sp_runtime::DispatchError>
	{
		if let Some(user_reward_info) = <Distributor<T>>::get(reward_id, account_id) {
			if let Some(reward_info) = <InitializeRewards<T>>::get(reward_id) {
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

				let reward_info = polkadex_primitives::rewards::RewardsInfoByAccount {
					claimed: user_reward_info.claim_amount.saturated_into::<u128>(),
					unclaimed: user_reward_info
						.total_reward_amount
						.saturating_sub(user_reward_info.claim_amount)
						.saturated_into::<u128>(),
					claimable: rewards_claimable,
				};
				return Ok(reward_info);
			}
		}
		Err(Error::<T>::UserNotEligible.into())
	}
}
