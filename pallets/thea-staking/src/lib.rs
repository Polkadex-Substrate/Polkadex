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

use polkadex_primitives::AccountId;
use sp_runtime::traits::{ Get};

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;
use crate::election::elect_relayers;
use crate::session::Exposure;

#[cfg(test)]
mod tests;
mod session;
mod election;

/// A type alias for the balance type from this pallet's point of view.
pub type BalanceOf<T> = <T as pallet_balances::Config>::Balance;
pub type BlockNumber<T> = <T as frame_system::Config>::BlockNumber;
pub type Network = u8;
pub type SessionIndex = u32;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_support::traits::NamedReservableCurrency;
    use frame_system::pallet_prelude::*;
    use polkadex_primitives::AccountId;
    use sp_runtime::traits::{ Zero};

    use crate::session::{Exposure, StakingLimits};

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

        /// StakingReserveIdentifier
        #[pallet::constant]
        type StakingReserveIdentifier: Get<<Self as pallet_balances::Config>::ReserveIdentifier>;

        /// Delay to prune oldest staking data
        type StakingDataPruneDelay: Get<SessionIndex>;


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
        // `on_finalize` is executed at the end of block after all extrinsic are dispatched.
        fn on_finalize(_n: T::BlockNumber) {
            // Perform necessary data/state clean up here.
        }

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
        #[pallet::call_index(0)]
        #[pallet::weight(10000)]
        pub fn set_staking_limits(origin: OriginFor<T>,  staking_limits: StakingLimits<BalanceOf<T>>) -> DispatchResult {
            ensure_root(origin)?;
            <Stakinglimits<T>>::put(staking_limits);
            Ok(())
        }

        #[pallet::call_index(1)]
        #[pallet::weight(10000)]
        /// Adds the sender as a candidate for election
        pub fn add_candidate(origin: OriginFor<T>,  own_stake: BalanceOf<T>, network: Network) -> DispatchResult {
            let candidate = ensure_signed(origin)?;
            let limits = <Stakinglimits<T>>::get();
            if own_stake < limits.mininum_relayer_stake {
                return Err(Error::<T>::StakingLimitsError.into())
            }
            ensure!(!<Candidates<T>>::contains_key(network,&candidate),Error::<T>::CandidateAlreadyRegistered);
            let mut exposure = Exposure::<T::AccountId,BalanceOf<T>>::default();
            exposure.add_own_stake(own_stake);
            // reserve own_stake
            pallet_balances::Pallet::<T>::reserve_named(&T::StakingReserveIdentifier::get(),&candidate,own_stake)?;
            <Candidates<T>>::insert(network,&candidate,exposure);
            Self::deposit_event(Event::<T>::CandidateRegistered{candidate,stake:own_stake});
            Ok(())
        }


        #[pallet::call_index(2)]
        #[pallet::weight(10000)]
        /// Increases the self stake of a candidate
        pub fn increase_own_stake(origin: OriginFor<T>,  additional_stake: BalanceOf<T>, network: Network) -> DispatchResult {
            let candidate = ensure_signed(origin)?;

            if let Some(mut exposure) = <Candidates<T>>::get(network,&candidate) {
                exposure.add_own_stake(additional_stake);
                // reserve own_stake
                pallet_balances::Pallet::<T>::reserve_named(&T::StakingReserveIdentifier::get(),&candidate,additional_stake)?;
                <Candidates<T>>::insert(network,&candidate,exposure);
                Self::deposit_event(Event::<T>::IncreasedCandidateStake{ candidate, stake:additional_stake});
            }else {
                return Err(Error::<T>::CandidateNotFound.into())
            }
            Ok(())
        }

        #[pallet::call_index(3)]
        #[pallet::weight(10000)]
        /// Nominate a candidate. If already nominating a candidate then calling this again will increase the stake
        pub fn nominate(origin: OriginFor<T>,  candidate: T::AccountId, network: Network, amount: BalanceOf<T>) -> DispatchResult {
            let nominator = ensure_signed(origin)?;
            let limits = <Stakinglimits<T>>::get();
            ensure!(amount>= limits.minimum_nominator_stake, Error::<T>::StakingLimitsError);

            if let Some(mut exposure) = <Candidates<T>>::get(network,&candidate) {
                ensure!(exposure.others.len()<limits.maximum_nominator_per_relayer as usize, Error::<T>::StakingLimitsError);
                exposure.nominate(&nominator, amount);
                // reserve own_stake
                pallet_balances::Pallet::<T>::reserve_named(&T::StakingReserveIdentifier::get(),&nominator,amount)?;
                <Candidates<T>>::insert(network,&candidate,exposure);
                Self::deposit_event(Event::<T>::Nominated{ candidate, stake:amount});
            }else {
                return Err(Error::<T>::CandidateNotFound.into())
            }
            Ok(())
        }

        #[pallet::call_index(4)]
        #[pallet::weight(10000)]
        /// Removes the nomination completely
        pub fn remove_nomination(origin: OriginFor<T>,  candidate: T::AccountId, network: Network, nominator_index: u32) -> DispatchResult {
            let nominator = ensure_signed(origin)?;

            if let Some(mut exposure) = <Candidates<T>>::get(network,&candidate) {
                let stake = exposure.remove_nominator(&nominator,nominator_index);
                // reserve own_stake
                let still_reserved = pallet_balances::Pallet::<T>::unreserve_named(
                    &T::StakingReserveIdentifier::get(),&nominator,stake);
                log::error!(target: "runtime::thea::staking", "unable to unreserve {:?} out of {:?}", still_reserved, stake);
                <Candidates<T>>::insert(network,&candidate,exposure);
                Self::deposit_event(Event::<T>::NominationRemoved{ candidate, nominator});
            }else {
                return Err(Error::<T>::CandidateNotFound.into())
            }
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
        /// New session is started
        NewSessionStarted {
            index: SessionIndex,
        },
        CandidateRegistered {
            candidate: T::AccountId,
            stake: BalanceOf<T>
        },
        IncreasedCandidateStake {
            candidate: T::AccountId,
            stake: BalanceOf<T>
        },
        Nominated{ candidate: T::AccountId,
            stake: BalanceOf<T>},
        NominationRemoved{
            candidate: T::AccountId,
            nominator: T::AccountId
        }
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Staking limits error
        StakingLimitsError,
        CandidateAlreadyRegistered,
        CandidateNotFound
    }

    // pallet::storage attributes allow for type-safe usage of the Substrate storage database,
    // so you can keep things around between blocks.
    #[pallet::storage]
    #[pallet::getter(fn staking_limits)]
    /// Currently active networks
    pub(super) type Stakinglimits<T: Config> = StorageValue<_, StakingLimits<BalanceOf<T>>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_networks)]
    /// Currently active networks
    pub(super) type ActiveNetworks<T: Config> = StorageValue<_, Vec<Network>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn active_relayers)]
    /// Currently active relayer set
    pub(super) type ActiveRelayers<T: Config> = StorageMap<_, Blake2_128Concat, Network, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn queued_relayers)]
    /// Upcoming relayer set
    pub(super) type QueuedRelayers<T: Config> = StorageMap<_, Blake2_128Concat, Network, Vec<T::AccountId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn staking_data)]
    /// Stores the economic conditions of a relayer and the contributions of their nominators for a
    /// given network and session index
    pub(super) type StakingData<T: Config> = StorageDoubleMap<_,
        Blake2_128Concat, SessionIndex,
         Blake2_128Concat, Network,
        Vec<(T::AccountId,Exposure<T::AccountId, BalanceOf<T>>)>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn candidates)]
    /// Stores the economic conditions of all candidates for relayers
    pub(super) type Candidates<T: Config> = StorageDoubleMap<_, Blake2_128Concat, Network,
        Blake2_128Concat, T::AccountId,
        Exposure<T::AccountId, BalanceOf<T>>, OptionQuery>;

    #[pallet::storage]
    #[pallet::getter(fn current_index)]
    /// Active Session Index
    pub(super) type CurrentIndex<T: Config> = StorageValue<_, SessionIndex, ValueQuery>;

}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
    // Add public immutables and private mutables.
	pub fn rotate_session(){
        let session_index = <CurrentIndex<T>>::get();
        log::trace!(target: "runtime::thea::staking", "rotating session {:?}", session_index);

        let active_networks = <ActiveNetworks<T>>::get();

        for network in active_networks {
            log::trace!(target: "runtime::thea::staking", "rotating for relayers of network {:?}", network);
            // 1. Move queued_relayers to active_relayers
            Self::move_queued_to_active(network);
            Self::compute_next_session(network, session_index);
        }
        // Increment SessionIndex
        let new_session_index = session_index.saturating_add(1);
        <CurrentIndex<T>>::put(new_session_index);
        Self::deposit_event(Event::NewSessionStarted {index: new_session_index})
	}

    pub fn move_queued_to_active(network: Network) {
        let queued = <QueuedRelayers<T>>::take(network);
        <ActiveRelayers<T>>::insert(network,queued);
    }

    pub fn compute_next_session(network: Network, expiring_session_index: SessionIndex){
        let session_in_consideration = expiring_session_index.saturating_add(2);
        log::trace!(target: "runtime::thea::staking", "computing relayers of session {:?}", session_in_consideration);
        // Get new queued_relayers and store them
        let candidates =  <Candidates<T>>::iter_prefix(network).collect::<Vec<(T::AccountId, Exposure<T::AccountId,BalanceOf<T>>)>>();
        let elected_relayers = elect_relayers::<T>(candidates);
        log::trace!(target: "runtime::thea::staking", "elected relayers of session {:?}", session_in_consideration);
        // Store their economic weights
        let relayers = elected_relayers.iter().map(| (relayer, _) | {
            relayer.clone()
        }).collect::<Vec<T::AccountId>>();
        <StakingData<T>>::insert(session_in_consideration,network, elected_relayers);
        <QueuedRelayers<T>>::insert(network, relayers);
        log::trace!(target: "runtime::thea::staking", "relayers of network {:?} queued for session {:?} ", network,session_in_consideration);
        // Delete oldest session's economic data from state
        let session_to_delete = session_in_consideration.saturating_sub(T::StakingDataPruneDelay::get());
        <StakingData<T>>::remove(session_to_delete,network);
        log::trace!(target: "runtime::thea::staking", "removing staking data of session {:?} and network {:?}", session_to_delete,network);
    }
}