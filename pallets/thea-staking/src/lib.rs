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

use frame_support::{ensure, pallet_prelude::*, traits::NamedReservableCurrency};
use sp_runtime::{
	traits::{Get, Saturating},
	DispatchError,
};
use sp_staking::{EraIndex, StakingInterface};
use sp_std::collections::btree_map::BTreeMap;

// Re-export pallet items so that they can be accessed from the crate namespace.
use crate::{
	election::elect_relayers,
	session::{Exposure, IndividualExposure},
};
pub use pallet::*;
use sp_std::vec::Vec;
use thea_primitives::{
	thea_types::{Network, OnSessionChange, SessionIndex},
	BLSPublicKey, TheaExtrinsicSubmitted,
};
use pallet_staking::EraPayout;
mod election;
// #[cfg(test)]
// mod mock;
mod session;
// #[cfg(test)]
// mod tests;

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
	use frame_support::{pallet_prelude::*, traits::NamedReservableCurrency};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::Zero;

	use crate::session::{Exposure, IndividualExposure, StakingLimits};
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
		) -> DispatchResult {
			ensure_root(origin)?;
			<Stakinglimits<T>>::put(staking_limits);
			Ok(())
		}

		/// Adds the sender as a candidate for election and to the waitlist for selection.
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

	// Reward Points for Relayers that submit extrinsic
	// (era, account_id) = Reward points
	#[pallet::storage]
	pub(super) type EraRewardPoints<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, T::AccountId, u32, ValueQuery>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	// Rewards author of extrinsic
	// # Parameters
	// * author: Author of the extrinsic
	pub fn reward_by_id(author: T::AccountId) {
		// Awarding the author of the extrinsic 50 points
		<EraRewardPoints<T>>::mutate(<CurrentIndex<T>>::get(), author, |current_points| {
			*current_points += 50;
		})
	}

	// Add public immutables and private mutables.
	pub fn rotate_session() {
		let session_index = <CurrentIndex<T>>::get();
		log::trace!(target: "runtime::thea::staking", "rotating session {:?}", session_index);
		let active_networks = <ActiveNetworks<T>>::get();
		// map to collect all active relayers to send to session change notifier
		let mut map: BTreeMap<Network, OnSessionChange<T::AccountId>> = BTreeMap::new();
		for network in active_networks {
			log::trace!(target: "runtime::thea::staking", "rotating for relayers of network {:?}", network);
			// 1. Move queued_relayers to active_relayers
			let active = Self::move_queued_to_active(network);
			map.insert(network, active);
			Self::compute_next_session(network, session_index);
		}
		// Increment SessionIndex
		let new_session_index = session_index.saturating_add(1);
		<CurrentIndex<T>>::put(new_session_index);
		T::SessionChangeNotifier::on_new_session(map);
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

	pub fn do_nominate(nominator: T::AccountId, candidate: T::AccountId) -> Result<(), Error<T>> {
		let mut nominator_exposure =
			<Stakers<T>>::get(&nominator).ok_or(Error::<T>::StakerNotFound)?;
		ensure!(nominator_exposure.backing.is_none(), Error::<T>::StakerAlreadyNominating);
		let network =
			<CandidateToNetworkMapping<T>>::get(&candidate).ok_or(Error::<T>::CandidateNotFound)?;
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
		ensure!(individual_exposure.value >= amount, Error::<T>::AmountIsGreaterThanBondedAmount);
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
		<StakingData<T>>::insert(session_in_consideration, network, elected_relayers);
		<QueuedRelayers<T>>::insert(network, relayers);
		log::trace!(target: "runtime::thea::staking", "relayers of network {:?} queued for session {:?} ", network,session_in_consideration);
		// Delete oldest session's economic data from state
		let session_to_delete =
			session_in_consideration.saturating_sub(T::StakingDataPruneDelay::get());
		<StakingData<T>>::remove(session_to_delete, network);
		log::trace!(target: "runtime::thea::staking", "removing staking data of session {:?} and network {:?}", session_to_delete,network);
	}
}

impl<T: Config> TheaExtrinsicSubmitted<T::AccountId> for Pallet<T> {
	fn thea_extrinsic_submitted(author: T::AccountId) {
		Self::reward_by_id(author);
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
	fn nominate(controller: Self::AccountId, validators: Vec<Self::AccountId>) -> DispatchResult {
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
