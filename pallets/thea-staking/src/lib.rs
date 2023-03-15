#![feature(drain_filter)]
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

use frame_support::{pallet_prelude::*, traits::ExistenceRequirement, PalletId};
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{AccountIdConversion, Saturating},
	Perbill, Percent, Permill, SaturatedConversion,
};
use sp_staking::{EraIndex, StakingInterface};
use sp_std::{borrow::ToOwned, collections::btree_map::BTreeMap};

// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::election::elect_relayers;
pub use pallet::*;
use pallet_staking::EraPayout;
use sp_runtime::traits::UniqueSaturatedInto;
use sp_std::{collections::btree_set::BTreeSet, vec::Vec};
use thea_primitives::{
	thea_types::{Network, OnSessionChange, SessionIndex},
	BLSPublicKey, TheaExtrinsicSubmitted,
};
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod election;
#[cfg(test)]
mod mock;
mod session;
#[cfg(test)]
mod tests;

#[derive(Debug, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct EraRewardPointTracker<Account> {
	pub total_points: u32,
	pub individual: BTreeMap<Account, u32>,
}

/// A type alias for the balance type from this pallet's point of view.
pub type BalanceOf<T> = <T as pallet_balances::Config>::Balance;
pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;

pub trait SessionChanged {
	type Network;
	type OnSessionChange;
	fn on_new_session(map: BTreeMap<Self::Network, Self::OnSessionChange>);
	fn set_new_networks(networks: sp_std::collections::btree_set::BTreeSet<Self::Network>);
}
// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
	use crate::session::{Exposure, IndividualExposure, StakingLimits};
	use frame_support::traits::{Currency, NamedReservableCurrency, ReservableCurrency};
	use frame_system::pallet_prelude::*;
	use polkadex_primitives::misbehavior::TheaMisbehavior;
	use scale_info::prelude::string::String;
	use sp_runtime::traits::Zero;
	// Import various types used to declare pallet in scope.
	use super::*;

	/// Our pallet's configuration trait. All our types and constants go in here. If the
	/// pallet is dependent on specific other pallets, then their configuration traits
	/// should be added to our implied traits list.
	///
	/// `frame_system::Config` should always be included.
	#[pallet::config]
	pub trait Config: pallet_balances::Config + frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Staking Session length
		#[pallet::constant]
		type SessionLength: Get<BlockNumber<Self>>;

		/// Delay in number of sessions before unbonded stake can be withdrawn
		#[pallet::constant]
		type UnbondingDelay: Get<SessionIndex>;

		/// Max number of unlocking chunks
		#[pallet::constant]
		type MaxUnlockChunks: Get<u32>;

		/// Candidate Bond
		#[pallet::constant]
		type CandidateBond: Get<BalanceOf<Self>>;

		/// StakingReserveIdentifier
		#[pallet::constant]
		type StakingReserveIdentifier: Get<<Self as pallet_balances::Config>::ReserveIdentifier>;

		/// Coeficient for moderate misbehavior slashing.
		/// Represents 1 to 100 percent of stake.
		/// Should remain within those bounds.
		#[pallet::constant]
		type ModerateSlashingCoeficient: Get<u8>;

		/// Coeficient for severe misbehavior slashing.
		/// Represents 1 to 100 percent of stake.
		/// Should remain within those bounds.
		#[pallet::constant]
		type SevereSlashingCoeficient: Get<u8>;

		/// Coeficient of slashed amount distibuted to each reporter
		/// Represents 1 to 100 percent of slashed amount.
		/// Should remain within those bounds.
		#[pallet::constant]
		type ReportersRewardCoeficient: Get<u8>;

		/// Threshold of reported relayers required for slashing to happen.
		/// Represents percentage of active vs reported relayers.
		#[pallet::constant]
		type SlashingThreshold: Get<u8>;

		/// Treasury PalletId
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		/// Delay to prune oldest staking data
		type StakingDataPruneDelay: Get<SessionIndex>;

		type SessionChangeNotifier: SessionChanged<
			Network = Network,
			OnSessionChange = OnSessionChange<Self::AccountId>,
		>;

		/// Governance origin to update the thea staking configuration
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		// Era Payout for set of Relayers
		type EraPayout: EraPayout<BalanceOf<Self>>;

		/// Native Currency handler
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Pallet implements [`Hooks`] trait to define some logic to execute in some context.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// `on_initialize` is executed at the beginning of the block before any extrinsic are
		// dispatched.
		//
		// This function must return the weight consumed by `on_initialize` and `on_finalize`.
		fn on_initialize(current_block_num: T::BlockNumber) -> Weight {
			if Self::should_end_session(current_block_num) {
				Self::rotate_session();
				Self::end_of_era();
				T::BlockWeights::get().max_block
			} else {
				// NOTE: the non-database part of the weight for `should_end_session(n)` is
				// included as weight for empty block, the database part is expected to be in
				// cache.
				Weight::zero()
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		//TODO: benchmark Thea Pallet. Issue #605
		/// Set Staking Limit
		/// Only Root User can call it
		///
		/// # Parameters
		///
		/// * `origin`: Root User
		/// * `staking_limit`: Limits of Staking algorithm.
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn set_staking_limits(
			origin: OriginFor<T>,
			staking_limits: StakingLimits<BalanceOf<T>>,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			<Stakinglimits<T>>::put(staking_limits);
			Ok(Pays::No.into())
		}

		/// Adds the sender as a candidate for election and to the   for selection.
		///
		/// # Parameters
		///
		/// * `network`: Network for which User wants to apply for candidature.
		/// * `bls_key`: BLS Key of Candidate.
		#[pallet::call_index(1)]
		#[pallet::weight(10000)]
		pub fn add_candidate(
			origin: OriginFor<T>,
			network: Network,
			bls_key: BLSPublicKey,
		) -> DispatchResult {
			let candidate = ensure_signed(origin)?;
			ensure!(
				!<Candidates<T>>::contains_key(network, &candidate),
				Error::<T>::CandidateAlreadyRegistered
			);

			let mut exposure = Exposure::<T>::new(bls_key);
			exposure.add_own_stake(T::CandidateBond::get());
			// reserve own_stake
			pallet_balances::Pallet::<T>::reserve_named(
				&T::StakingReserveIdentifier::get(),
				&candidate,
				T::CandidateBond::get(),
			)?;
			<Candidates<T>>::insert(network, &candidate, exposure);
			<CandidateToNetworkMapping<T>>::insert(&candidate, network);
			Self::deposit_event(Event::<T>::CandidateRegistered {
				candidate,
				stake: T::CandidateBond::get(),
			});
			Ok(())
		}

		/// Nominates candidate for Active Relayer Set for provided network.
		/// Can be called by Nominator who already staked.
		///
		///# Parameters
		///
		/// * `candidate`: Candidate to be nominated.
		#[pallet::call_index(2)]
		#[pallet::weight(10000)]
		pub fn nominate(origin: OriginFor<T>, candidate: T::AccountId) -> DispatchResult {
			let nominator = ensure_signed(origin)?;
			Self::do_nominate(nominator, candidate)?;
			Ok(())
		}

		/// Locks Balance of Nominator for Staking purpose.
		///
		/// # Parameters
		///
		/// `amount`: Amount to be locked.
		#[pallet::call_index(3)]
		#[pallet::weight(10000)]
		pub fn bond(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let nominator = ensure_signed(origin)?;
			Self::do_bond(nominator, amount)?;
			Ok(())
		}

		/// Unbonds provided amount which Nominator wants to unlock.
		///
		///# Parameters
		///
		/// `amount`: Amount which User wants to Unbond.
		#[pallet::call_index(4)]
		#[pallet::weight(10000)]
		pub fn unbond(origin: OriginFor<T>, amount: BalanceOf<T>) -> DispatchResult {
			let nominator = ensure_signed(origin)?;
			Self::do_unbond(nominator, amount)?;
			Ok(())
		}

		/// Withdraws Unlocked funds
		#[pallet::call_index(5)]
		#[pallet::weight(10000)]
		pub fn withdraw_unbonded(origin: OriginFor<T>) -> DispatchResult {
			let nominator = ensure_signed(origin)?;

			Self::do_withdraw_unbonded(nominator)?;
			Ok(())
		}

		//TODO: After removing candidature, Candidate can't claim back bonded amount. #607

		/// Removes Candidate from Active/Waiting Set.
		///
		/// # Parameters
		///
		/// `network`: Network from which Candidate will be removed.
		#[pallet::call_index(6)]
		#[pallet::weight(10000)]
		pub fn remove_candidate(origin: OriginFor<T>, network: Network) -> DispatchResult {
			let candidate = ensure_signed(origin)?;

			let exposure =
				<Candidates<T>>::take(network, &candidate).ok_or(Error::<T>::CandidateNotFound)?;

			<InactiveCandidates<T>>::insert(network, &candidate, exposure);
			Self::deposit_event(Event::<T>::OutgoingCandidateAdded { candidate });
			Ok(())
		}

		/// Adds a new network, it will also update the thea pallet's storage
		///
		/// # Parameters
		///
		/// `network`: Network identifier to add
		#[pallet::call_index(7)]
		#[pallet::weight(10000)]
		pub fn add_network(origin: OriginFor<T>, network: Network) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			Self::do_add_new_network(network);
			Self::deposit_event(Event::<T>::NetworkAdded { network });
			Ok(())
		}

		/// Removes a network, it will also update the thea pallet's storage
		///
		/// # Parameters
		///
		/// `network`: Network identifier to add
		#[pallet::call_index(8)]
		#[pallet::weight(10000)]
		pub fn remove_network(origin: OriginFor<T>, network: Network) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			Self::do_remove_network(network);
			Self::deposit_event(Event::<T>::NetworkRemoved { network });
			Ok(())
		}

		/// Allows active relayers report one another for free of any predefined misbehavior
		/// If number of reporters is >= of given coeficient - slashing for pre-set coeficient
		/// happens If reporter is part of active set - this call's fee is not apply
		/// Full weight is payed on error
		/// # Params
		/// * network_id - identifier of network where ofence was registered
		/// * offender - ID of relayer commited ofence
		/// * offence - type of registere ofence
		#[pallet::call_index(9)]
		#[pallet::weight(1_000_000)]
		pub fn report_offence(
			origin: OriginFor<T>,
			network_id: u8,
			offender: T::AccountId,
			offence: TheaMisbehavior,
		) -> DispatchResultWithPostInfo {
			let reporter = ensure_signed(origin)?;
			// make sure it's active relayer reporting
			ensure!(
				<ActiveRelayers<T>>::get(network_id).iter().any(|(r, _)| r.eq(&reporter)),
				Error::<T>::NotAnActiveRelayer
			);
			// check for re-submit
			//FIXME: should we charge for sequential report of same offence by same reporter?
			ensure!(
				!<ReportedOffenders<T>>::get(offender.clone(), offence)
					.unwrap_or_default()
					.contains(&reporter),
				Error::<T>::RepeatedReport
			);
			ensure!(
				!<CommitedSlashing<T>>::contains_key(&offender),
				Error::<T>::SlashingInProgress
			);
			// check if coeficient treshold reached and act
			let threshold = Self::threshold_slashing_coeficient();
			let active_relayers = <ActiveRelayers<T>>::get(network_id).len();
			if let Some(reported) = <ReportedOffenders<T>>::get(offender.clone(), offence) {
				if reported.len() + 1 + (threshold as usize) >= active_relayers {
					// slash
					// <CommitedSlashing<T>> -> store commitment to slash so it can be applyed on
					let coeficient = match offence {
						// Severe
						TheaMisbehavior::UnattendedKeygen | TheaMisbehavior::UnattendedOffline =>
							Self::severe_slashing_coeficient(),
						// Moderate
						_ => Self::moderate_slashing_coeficient(),
					};
					// at most 100% will be slashed
					// era end FIXME: make sure total slash <= offender's stake + reward?
					<CommitedSlashing<T>>::mutate(&offender, |current_slashing| {
						let new_percentage = current_slashing.0 + coeficient;
						let actual_percentage =
							if new_percentage >= 100 { 100 } else { new_percentage };
						current_slashing.0 = actual_percentage;
						current_slashing.1.insert(reporter.clone());
						for previous_reporter in reported {
							current_slashing.1.insert(previous_reporter);
						}
					});
				} else {
					// extend storage
					<ReportedOffenders<T>>::mutate(offender.clone(), offence, |offences| {
						if let Some(offences) = offences {
							offences.push(reporter.clone());
						}
					});
				}
			} else {
				// register first one
				<ReportedOffenders<T>>::insert(
					offender.clone(),
					offence,
					[reporter.clone()].to_vec(),
				);
			}
			Self::deposit_event(Event::<T>::OffenceReported { offender, reporter, offence });

			Ok(Pays::No.into())
		}

		/// Pays the stakers of a Relayer for a given Session
		///
		/// # Parameters
		///
		/// `session`: SessionIndex of the Session to be paid out for
		#[pallet::call_index(10)]
		#[pallet::weight(10000)]
		pub fn stakers_payout(origin: OriginFor<T>, session: SessionIndex) -> DispatchResult {
			let staker = ensure_signed(origin)?;
			Self::do_stakers_payout(staker.clone(), session)?;
			Self::deposit_event(Event::<T>::StakerPayedOut { staker, session }.into());
			Ok(())
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	/// This attribute generate the function `deposit_event` to deposit one of this pallet event,
	/// it is optional, it is also possible to provide a custom implementation.
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New Network Added
		NetworkAdded {
			network: Network,
		},
		/// New Network Removed
		NetworkRemoved {
			network: Network,
		},
		/// New session is started
		NewSessionStarted {
			index: SessionIndex,
		},
		CandidateRegistered {
			candidate: T::AccountId,
			stake: BalanceOf<T>,
		},
		OutgoingCandidateAdded {
			candidate: T::AccountId,
		},
		IncreasedCandidateStake {
			candidate: T::AccountId,
			stake: BalanceOf<T>,
		},
		Nominated {
			candidate: T::AccountId,
			nominator: T::AccountId,
		},
		Unbonded {
			candidate: Option<T::AccountId>,
			nominator: T::AccountId,
			amount: BalanceOf<T>,
		},
		Bonded {
			candidate: T::AccountId,
			nominator: T::AccountId,
			amount: BalanceOf<T>,
		},
		BondsWithdrawn {
			nominator: T::AccountId,
			amount: BalanceOf<T>,
		},

		/// Misconfigured Coeficient
		MisconfiguredCoeficient(String),

		/// Active relayer reported misbehavior
		OffenceReported {
			offender: T::AccountId,
			reporter: T::AccountId,
			offence: TheaMisbehavior,
		},

		/// Cleaned up slashes
		SlashesCleaned(u32),

		/// Slashed offender for percend based on commited offence
		Slashed {
			offender: T::AccountId,
			amount: BalanceOf<T>,
		},

		/// Reward distibuted to reporter relayer for correct offence report
		ReportRewarded {
			reporter: T::AccountId,
			amount: BalanceOf<T>,
		},

		/// Failed to transfer slashed amount from offender's account
		SlashingFailed {
			offender: T::AccountId,
		},

		/// Staker got payed out for session ID
		StakerPayedOut {
			staker: T::AccountId,
			session: SessionIndex,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Staking limits error
		StakingLimitsError,
		CandidateAlreadyRegistered,
		CandidateNotFound,
		NominatorNotFound,
		UnbondChunkLimitReached,
		CandidateNotReadyToBeUnbonded,
		StakerNotFound,
		StakerAlreadyNominating,
		CandidateAlreadyNominated,
		OnlyOneRelayerCanBeNominated,
		StashAndControllerMustBeSame,
		AmountIsGreaterThanBondedAmount,
		/// Repeating report of same offence is prohibited
		RepeatedReport,
		/// Not a member of active relayers
		NotAnActiveRelayer,
		/// Offender already scheduled for slashing
		SlashingInProgress,
	}

	// pallet::storage attributes allow for type-safe usage of the Substrate storage database,
	// so you can keep things around between blocks.
	#[pallet::storage]
	#[pallet::getter(fn staking_limits)]
	/// Currently active networks
	pub(super) type Stakinglimits<T: Config> =
		StorageValue<_, StakingLimits<BalanceOf<T>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn active_networks)]
	/// Currently active networks
	pub(super) type ActiveNetworks<T: Config> =
		StorageValue<_, sp_std::collections::btree_set::BTreeSet<Network>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn active_relayers)]
	/// Currently active relayer set
	pub(super) type ActiveRelayers<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, Vec<(T::AccountId, BLSPublicKey)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn queued_relayers)]
	/// Upcoming relayer set
	pub(super) type QueuedRelayers<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, Vec<(T::AccountId, BLSPublicKey)>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn staking_data)]
	/// Stores the economic conditions of a relayer and the contributions of their nominators for a
	/// given network and session index
	pub(super) type StakingData<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		SessionIndex,
		Blake2_128Concat,
		Network,
		Vec<(T::AccountId, Exposure<T>)>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn stakers)]
	/// Staker account to candidate backing map
	/// NOTE: One staker account can only back one candidate in one network
	pub(super) type Stakers<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		IndividualExposure<T, T::AccountId>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn candidates)]
	/// Stores the economic conditions of all candidates for relayers
	pub(super) type Candidates<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Network,
		Blake2_128Concat,
		T::AccountId,
		Exposure<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn candidates_to_network)]
	/// Mapping from candidate to network
	pub(super) type CandidateToNetworkMapping<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Network, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn outgoing_candidates)]
	/// Stores the economic conditions of all candidates who requested to leave the election process
	pub(super) type InactiveCandidates<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Network,
		Blake2_128Concat,
		T::AccountId,
		Exposure<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn disabled_candidates)]
	/// Stores the economic conditions of all candidates who are disabled for misbehaviour
	pub(super) type DisabledCandidates<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Network,
		Blake2_128Concat,
		T::AccountId,
		Exposure<T>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn current_index)]
	/// Active Session Index
	pub(super) type CurrentIndex<T: Config> = StorageValue<_, SessionIndex, ValueQuery>;

	/// Reported offenders in current era
	#[pallet::storage]
	#[pallet::getter(fn reported_offenders)]
	pub(super) type ReportedOffenders<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		// Offender ID
		T::AccountId,
		Blake2_128Concat,
		// Commited misbehavior
		TheaMisbehavior,
		// Reporters
		Vec<T::AccountId>,
		OptionQuery,
	>;

	/// Summ of commited slashing for each relayer in current era
	/// Calculated based on reports from <ReportedOffenders<T>> storage
	/// Represents how many percent will be slashed from given receiver's stake on era end
	/// BTreeSet are reporters to be rewarded for reporting misbehavior
	#[pallet::storage]
	#[pallet::getter(fn commited_slashing)]
	pub(super) type CommitedSlashing<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, (u8, BTreeSet<T::AccountId>), ValueQuery>;

	// Reward Points for Relayers that submit extrinsic
	// (era, account_id) = Reward points
	#[pallet::storage]
	pub(super) type EraRewardPoints<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, EraRewardPointTracker<T::AccountId>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn era_reward_payout)]
	// Stores the Total Reward Payout for a Session
	pub(super) type EraRewardPayout<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	/// Stores the Total Active Stake for a given Session
	pub(super) type TotalSessionStake<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, BalanceOf<T>, ValueQuery>;

	#[pallet::storage]
	/// Stores the Total Elected Relayers for a given Session
	pub(super) type TotalElectedRelayers<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, Vec<(T::AccountId, Exposure<T>)>, ValueQuery>;

	// The main implementation block for the pallet. Functions here fall into three broad
	// categories:
	// - Public interface. These are functions that are `pub` and generally fall into inspector
	// functions that do not write to storage and operation functions that do.
	// - Private functions. These are your usual private utilities unavailable to other pallets.
	impl<T: Config> Pallet<T> {
		// Rewards author of extrinsic
		// # Parameters
		// * author: Author of the extrinsic
		pub fn reward_by_id(author: T::AccountId, _bit_map: u128, _active_set: Vec<T::AccountId>) {
			<EraRewardPoints<T>>::mutate(<CurrentIndex<T>>::get(), |tracker| {
				if let Some(tracker) = tracker {
					tracker.total_points += 50;
					if let Some(existing_points) = tracker.individual.get_mut(&author) {
						*existing_points += 50;
					} else {
						tracker.individual.insert(author, 50);
					}
				} else {
					let mut btree_map: BTreeMap<T::AccountId, u32> = BTreeMap::new();
					btree_map.insert(author, 50);
					let points_tracker: EraRewardPointTracker<T::AccountId> =
						EraRewardPointTracker { total_points: 50, individual: btree_map };
					*tracker = Some(points_tracker);
				}
			});
		}

		pub fn end_of_era() {
			// FIXME: Need to remove hardcoded value
			let era = <CurrentIndex<T>>::get();
			let total_issuance: u32 = T::Currency::total_issuance().unique_saturated_into();
			let eras_total_stake = <TotalSessionStake<T>>::get(era);
			// FIXME: This hardcoded value needs to be updated
			let (era_payout, _rest) =
				T::EraPayout::era_payout(eras_total_stake, total_issuance.into(), 7200);
			<EraRewardPayout<T>>::insert(era, era_payout);
		}

		pub fn eras_total_stake() -> BalanceOf<T> {
			// FIXME: This should be active relayers for a given an era
			let _active_relayers = <ActiveRelayers<T>>::get(1);
			let staking_data = <StakingData<T>>::get(<CurrentIndex<T>>::get(), 0);
			let mut total_stake: BalanceOf<T> = 0_u32.into();
			for (_, exposure) in staking_data {
				let stake = exposure.total;
				total_stake += stake;
			}
			total_stake
		}

		pub fn do_stakers_payout(stash_account: T::AccountId, era: SessionIndex) -> DispatchResult {
			let total_payout = <EraRewardPayout<T>>::get(era);
			let mut relayer_part: Perbill = Perbill::default();
			if let Some(rewards) = <EraRewardPoints<T>>::get(era) {
				relayer_part = Perbill::from_rational(
					*rewards.individual.get(&stash_account).unwrap(),
					rewards.total_points,
				);
			}
			let relayer_payout = relayer_part * total_payout;
			// 1. Calculate Nominators Payout
			// Get Exposure for the given relayer
			let total_elected_relayers = <TotalElectedRelayers<T>>::get(era);
			let exposure = total_elected_relayers
				.iter()
				.filter(|(account_id, _)| *account_id == stash_account)
				.fold(Exposure::new(BLSPublicKey([0_u8; 192])), |_, i| i.1.to_owned());
			let total_stake = exposure.total;
			let individual_part = Perbill::from_rational(exposure.individual, total_stake);
			let individual_payout = individual_part * relayer_payout;
			// panic!("Alice individual payout: {:?}", total_payout);
			// Mint it to the Relayer
			let individual_payout: u32 = individual_payout.unique_saturated_into();
			T::Currency::deposit_into_existing(&stash_account, individual_payout.into())?;

			for nominator in exposure.stakers {
				// Get Exposure of Stakers
				if let Some(nominator_exposure) = <Stakers<T>>::get(&nominator) {
					let nominator_stake = nominator_exposure.value;
					let nominator_part = Perbill::from_rational(nominator_stake, total_stake);
					// TODO: Check if backing is the same
					let nominator_payout = nominator_part * relayer_payout;
					let nominator_payout: u32 = nominator_payout.unique_saturated_into();
					// Mint Rewards for Nominators
					T::Currency::deposit_into_existing(&nominator, nominator_payout.into())?;
				}
			}
			Ok(())
		}

		// Add public immutables and private mutables.
		pub fn rotate_session() {
			let session_index = <CurrentIndex<T>>::get();
			log::trace!(target: "runtime::thea::staking", "rotating session {:?}", session_index);
			let active_networks = <ActiveNetworks<T>>::get();
			for (offender, (percent, reporters)) in <CommitedSlashing<T>>::iter() {
				if let Some(net) =
					active_networks.iter().find(|n| <Candidates<T>>::contains_key(n, &offender))
				{
					if let Some(to_slash) = <Candidates<T>>::get(net, &offender) {
						// total amount transfered to treasury - used for reporters award
						// calculations
						let mut total_slashed = BalanceOf::<T>::zero();
						let actual_percent = Percent::from_percent(percent);
						// slashing relayer's individual stake
						let amount: BalanceOf<T> = actual_percent * to_slash.individual;
						// TODO: where to transfer? % > Treasury && % > to reporters
						if <pallet_balances::Pallet<T> as Currency<_>>::transfer(
							&offender,
							&T::TreasuryPalletId::get().into_account_truncating(),
							amount,
							ExistenceRequirement::KeepAlive,
						)
						.is_ok()
						{
							total_slashed = total_slashed.saturating_add(amount);
							Self::deposit_event(Event::Slashed { offender, amount });
						}
						// slash stakers / nominators
						for nominator in to_slash.stakers.iter() {
							if let Some(individual_nominator) = <Stakers<T>>::get(nominator) {
								let nominator_amount_individual: BalanceOf<T> =
									actual_percent * individual_nominator.value;
								if <pallet_balances::Pallet<T> as Currency<_>>::transfer(
									nominator,
									&T::TreasuryPalletId::get().into_account_truncating(),
									nominator_amount_individual,
									ExistenceRequirement::KeepAlive,
								)
								.is_ok()
								{
									total_slashed =
										total_slashed.saturating_add(nominator_amount_individual);
									Self::deposit_event(Event::Slashed {
										offender: nominator.to_owned(),
										amount: nominator_amount_individual,
									});
								}
							} else {
								// we signal issue with staker slashing via Event
								Self::deposit_event(Event::SlashingFailed {
									offender: nominator.to_owned(),
								});
							}
						}
						// distribute to reporters
						let reporter_percent =
							Percent::from_percent(T::ReportersRewardCoeficient::get());
						let reporter_award: BalanceOf<T> = reporter_percent * total_slashed;
						let reporter_individual_part =
							Permill::from_rational(1, reporters.len() as u32);
						let reporter_individual_award: BalanceOf<T> =
							reporter_individual_part * reporter_award;
						for reporter in reporters.into_iter() {
							if <pallet_balances::Pallet<T> as Currency<_>>::transfer(
								&T::TreasuryPalletId::get().into_account_truncating(),
								&reporter,
								reporter_individual_award,
								ExistenceRequirement::KeepAlive,
							)
							.is_ok()
							{
								Self::deposit_event(Event::ReportRewarded {
									reporter,
									amount: reporter_individual_award,
								});
							}
						}
					} else {
						Self::deposit_event(Event::SlashingFailed { offender });
					}
				}
			}

			// reset of slashed store and reports
			// max active validators count
			let max_ops: u32 = active_networks
				.iter()
				.fold(0, |acc, network| acc + <ActiveRelayers<T>>::get(network).len())
				.saturated_into();
			let sp_io::MultiRemovalResults { unique, .. } =
				<CommitedSlashing<T>>::clear(max_ops, None);
			let unique_reports = unique;
			let sp_io::MultiRemovalResults { unique, .. } =
				<ReportedOffenders<T>>::clear(max_ops, None);
			Self::deposit_event(Event::SlashesCleaned((unique + unique_reports).saturated_into()));
			// map to collect all active relayers to send to session change notifier
			let mut map: BTreeMap<Network, OnSessionChange<T::AccountId>> = BTreeMap::new();
			for network in active_networks {
				log::trace!(target: "runtime::thea::staking", "rotating for relayers of network {:?}", network);
				let active = Self::move_queued_to_active(network);
				map.insert(network, active);
				Self::compute_next_session(network, session_index);
			}
			// Increment SessionIndex
			let new_session_index = session_index.saturating_add(1);
			<CurrentIndex<T>>::put(new_session_index);
			T::SessionChangeNotifier::on_new_session(map);
			// TODO: implement slashing
			Self::deposit_event(Event::NewSessionStarted { index: new_session_index })
		}

		pub fn do_add_new_network(network: Network) {
			let mut active_networks = <ActiveNetworks<T>>::get();
			if !active_networks.contains(&network) {
				active_networks.insert(network);
				<ActiveNetworks<T>>::put(&active_networks);
				T::SessionChangeNotifier::set_new_networks(active_networks);
			}
		}

		pub fn do_remove_network(network: Network) {
			let mut active_networks = <ActiveNetworks<T>>::get();
			if active_networks.remove(&network) {
				<ActiveNetworks<T>>::put(&active_networks);
				T::SessionChangeNotifier::set_new_networks(active_networks);
			}
		}
		// FIXME: The current implementation allows Nominators to nominate only one relayer
		// with the entire stake that has been bonded
		pub fn do_nominate(
			nominator: T::AccountId,
			candidate: T::AccountId,
		) -> Result<(), Error<T>> {
			let mut nominator_exposure =
				<Stakers<T>>::get(&nominator).ok_or(Error::<T>::StakerNotFound)?;
			ensure!(nominator_exposure.backing.is_none(), Error::<T>::StakerAlreadyNominating);
			let network = <CandidateToNetworkMapping<T>>::get(&candidate)
				.ok_or(Error::<T>::CandidateNotFound)?;
			let mut exposure =
				<Candidates<T>>::get(network, &candidate).ok_or(Error::<T>::CandidateNotFound)?;

			ensure!(!exposure.stakers.contains(&nominator), Error::<T>::CandidateAlreadyNominated);
			exposure.stakers.insert(nominator.clone());
			exposure.total = exposure.total.saturating_add(nominator_exposure.value);
			nominator_exposure.backing = Some((network, candidate.clone()));
			<Stakers<T>>::insert(&nominator, nominator_exposure);
			<Candidates<T>>::insert(network, &candidate, exposure);
			Self::deposit_event(Event::<T>::Nominated { candidate, nominator });
			Ok(())
		}

		pub fn do_withdraw_unbonded(nominator: T::AccountId) -> Result<(), Error<T>> {
			if let Some(mut exposure) = <Stakers<T>>::get(&nominator) {
				let amount: BalanceOf<T> = exposure.withdraw_unbonded(Self::current_index());
				let _ = pallet_balances::Pallet::<T>::unreserve_named(
					&T::StakingReserveIdentifier::get(),
					&nominator,
					amount,
				);
				<Stakers<T>>::insert(&nominator, exposure);
				Self::deposit_event(Event::<T>::BondsWithdrawn { nominator, amount });
			} else {
				return Err(Error::<T>::CandidateNotFound)
			}
			Ok(())
		}

		pub fn do_unbond(nominator: T::AccountId, amount: BalanceOf<T>) -> Result<(), Error<T>> {
			let mut individual_exposure =
				<Stakers<T>>::get(&nominator).ok_or(Error::<T>::StakerNotFound)?;
			ensure!(
				individual_exposure.value >= amount,
				Error::<T>::AmountIsGreaterThanBondedAmount
			);
			if let Some((network, candidate)) = individual_exposure.backing.as_ref() {
				if let Some(mut exposure) = <Candidates<T>>::get(network, candidate) {
					exposure.total = exposure.total.saturating_sub(amount);
					if individual_exposure.value == amount {
						exposure.stakers.remove(&nominator);
					}
					<Candidates<T>>::insert(network, candidate, exposure);
					Self::deposit_event(Event::<T>::Unbonded {
						candidate: Some(candidate.clone()),
						nominator: nominator.clone(),
						amount,
					});
				}
			}
			if individual_exposure.value == amount {
				individual_exposure.backing = None;
			}
			individual_exposure
				.unbond(amount, Self::current_index().saturating_add(T::UnbondingDelay::get()));

			<Stakers<T>>::insert(&nominator, individual_exposure);
			Self::deposit_event(Event::<T>::Unbonded { candidate: None, nominator, amount });
			Ok(())
		}

		pub fn do_bond(nominator: T::AccountId, amount: BalanceOf<T>) -> Result<(), DispatchError> {
			let limits = <Stakinglimits<T>>::get();
			//FIXME: minimum_nominator_stake should be only checked once
			ensure!(amount >= limits.minimum_nominator_stake, Error::<T>::StakingLimitsError);
			if let Some(mut individual_exposure) = <Stakers<T>>::get(&nominator) {
				if let Some((network, candidate)) = individual_exposure.backing {
					if let Some(mut exposure) = <Candidates<T>>::get(network, &candidate) {
						exposure.total = exposure.total.saturating_add(amount);
						exposure.stakers.insert(nominator.clone());
						// reserve stake
						pallet_balances::Pallet::<T>::reserve_named(
							&T::StakingReserveIdentifier::get(),
							&nominator,
							amount,
						)?;
						<Candidates<T>>::insert(network, &candidate, exposure);
						Self::deposit_event(Event::<T>::Bonded { candidate, nominator, amount });
					} else {
						return Err(Error::<T>::CandidateNotFound.into())
					}
				} else {
					pallet_balances::Pallet::<T>::reserve_named(
						&T::StakingReserveIdentifier::get(),
						&nominator,
						amount,
					)?;
					individual_exposure.value += amount;
					<Stakers<T>>::insert(&nominator, individual_exposure);
				}
			} else {
				// reserve stake
				pallet_balances::Pallet::<T>::reserve_named(
					&T::StakingReserveIdentifier::get(),
					&nominator,
					amount,
				)?;
				<Stakers<T>>::insert(
					&nominator,
					IndividualExposure {
						who: nominator.clone(),
						value: amount,
						backing: None,
						unlocking: Vec::new(),
					},
				)
			}
			Ok(())
		}

		pub fn move_queued_to_active(network: Network) -> OnSessionChange<T::AccountId> {
			let queued = <QueuedRelayers<T>>::take(network);
			<ActiveRelayers<T>>::insert(network, queued.clone());
			let mut vec_of_bls_keys: Vec<BLSPublicKey> = Vec::new();
			let mut account_ids: Vec<T::AccountId> = Vec::new();
			for (account_id, bls_key) in queued {
				vec_of_bls_keys.push(bls_key);
				account_ids.push(account_id);
			}
			(vec_of_bls_keys, account_ids)
		}

		pub fn get_queued_relayers_bls_keys(network: Network) -> Vec<BLSPublicKey> {
			<QueuedRelayers<T>>::get(network)
				.iter()
				.map(|(_, b)| *b)
				.collect::<Vec<BLSPublicKey>>()
		}

		pub fn compute_next_session(network: Network, expiring_session_index: SessionIndex) {
			// Wait wtf, why is this 2? Fuck
			// This affects genesis session, fine
			let session_in_consideration = expiring_session_index.saturating_add(2);
			log::trace!(target: "runtime::thea::staking", "computing relayers of session {:?}", session_in_consideration);
			// Get new queued_relayers and store them
			let candidates =
				<Candidates<T>>::iter_prefix(network).collect::<Vec<(T::AccountId, Exposure<T>)>>();
			let elected_relayers = elect_relayers::<T>(candidates);
			log::trace!(target: "runtime::thea::staking", "elected relayers of session {:?}", session_in_consideration);
			// Store their economic weights
			let relayers = elected_relayers
				.iter()
				.map(|(relayer, exp)| (relayer.clone(), exp.bls_pub_key))
				.collect::<Vec<(T::AccountId, BLSPublicKey)>>();

			// Calculate the total stake for these relayers
			let total_stake = elected_relayers
				.iter()
				.map(|(_, exp)| exp.total)
				.fold(0_u32.into(), |sum: BalanceOf<T>, i| sum.saturating_add(i));
			<TotalSessionStake<T>>::mutate(session_in_consideration, |existing_stake| {
				existing_stake.saturating_add(total_stake);
			});
			<TotalElectedRelayers<T>>::mutate(session_in_consideration, |list_of_relayers| {
				list_of_relayers.extend(elected_relayers.clone());
			});
			<StakingData<T>>::insert(session_in_consideration, network, elected_relayers);
			<QueuedRelayers<T>>::insert(network, relayers);
			log::trace!(target: "runtime::thea::staking", "relayers of network {:?} queued for session {:?} ", network,session_in_consideration);
			// Delete oldest session's economic data from state
			let session_to_delete =
				session_in_consideration.saturating_sub(T::StakingDataPruneDelay::get());
			<StakingData<T>>::remove(session_to_delete, network);
			log::trace!(target: "runtime::thea::staking", "removing staking data of session {:?} and network {:?}", session_to_delete,network);
		}

		// making sure we're not exceeding 100% and not below 1%
		fn moderate_slashing_coeficient() -> u8 {
			const FIXED_MODERATE: u8 = 5;
			let set = T::ModerateSlashingCoeficient::get();
			if !(1..=100).contains(&set) {
				Self::deposit_event(Event::<T>::MisconfiguredCoeficient("Moderate".into()));
				FIXED_MODERATE
			} else {
				set
			}
		}

		// making sure we're not exceeding 100% and not below 1%
		fn severe_slashing_coeficient() -> u8 {
			const FIXED_SEVERE: u8 = 20;
			let set = T::SevereSlashingCoeficient::get();
			if !(1..=100).contains(&set) {
				Self::deposit_event(Event::<T>::MisconfiguredCoeficient("Severe".into()));
				FIXED_SEVERE
			} else {
				set
			}
		}

		// making sure we're not exceeding 100% and not below 1%
		fn threshold_slashing_coeficient() -> u8 {
			const FIXED_THRESHOLD: u8 = 60;
			let set = T::SlashingThreshold::get();
			if !(1..=100).contains(&set) {
				Self::deposit_event(Event::<T>::MisconfiguredCoeficient("Threshold".into()));
				FIXED_THRESHOLD
			} else {
				set
			}
		}
	}

	impl<T: Config> TheaExtrinsicSubmitted<T::AccountId> for Pallet<T> {
		fn thea_extrinsic_submitted(
			author: T::AccountId,
			bit_map: u128,
			active_set: Vec<T::AccountId>,
		) {
			Self::reward_by_id(author, bit_map, active_set);
		}
	}

	/// Staking Interface is required to Nomination Pools pallet to work
	impl<T: Config> StakingInterface for Pallet<T> {
		type Balance = T::Balance;
		type AccountId = T::AccountId;

		fn minimum_bond() -> Self::Balance {
			T::CandidateBond::get()
		}

		fn bonding_duration() -> EraIndex {
			T::UnbondingDelay::get()
		}

		fn current_era() -> EraIndex {
			<CurrentIndex<T>>::get()
		}

		fn active_stake(staker: &Self::AccountId) -> Option<Self::Balance> {
			if let Some(individual_exposure) = <Stakers<T>>::get(staker) {
				return Some(individual_exposure.value)
			}
			None
		}

		fn total_stake(staker: &Self::AccountId) -> Option<Self::Balance> {
			if let Some(individual_exposure) = <Stakers<T>>::get(staker) {
				let mut total: BalanceOf<T> = individual_exposure.value;
				for chunk in individual_exposure.unlocking {
					total = total.saturating_add(chunk.value)
				}
				return Some(total)
			}
			None
		}

		fn bond(
			stash: Self::AccountId,
			controller: Self::AccountId,
			value: Self::Balance,
			_payee: Self::AccountId,
		) -> DispatchResult {
			ensure!(stash == controller, Error::<T>::StashAndControllerMustBeSame);
			Pallet::<T>::do_bond(stash, value)?;
			Ok(())
		}

		/// NOTE: Thea staking doesnt have the concept of controller-stash pair.
		/// So controller and stash should be same.
		fn nominate(
			controller: Self::AccountId,
			validators: Vec<Self::AccountId>,
		) -> DispatchResult {
			ensure!(validators.len() == 1, Error::<T>::OnlyOneRelayerCanBeNominated);
			Pallet::<T>::do_nominate(controller, validators[0].clone())?;
			Ok(())
		}

		fn chill(_controller: Self::AccountId) -> DispatchResult {
			// There is no concept of chill in Thea Staking.
			Ok(())
		}

		fn bond_extra(stash: Self::AccountId, extra: Self::Balance) -> DispatchResult {
			Pallet::<T>::do_bond(stash, extra)?;
			Ok(())
		}

		fn unbond(stash: Self::AccountId, value: Self::Balance) -> DispatchResult {
			Pallet::<T>::do_unbond(stash, value)?;
			Ok(())
		}

		fn withdraw_unbonded(
			stash: Self::AccountId,
			_num_slashing_spans: u32,
		) -> Result<bool, DispatchError> {
			// TODO: Figure out whether it is right to return false.
			Pallet::<T>::do_withdraw_unbonded(stash)?;
			Ok(false)
		}
	}
}
