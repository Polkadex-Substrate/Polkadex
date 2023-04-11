// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
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

use parity_scale_codec::{Encode, MaxEncodedLen};

use frame_support::{
	log,
	traits::{Get, OneSessionHandler},
	BoundedSlice, BoundedVec, Parameter,
};

use sp_runtime::{
	generic::DigestItem,
	traits::{IsMember, Member},
	RuntimeAppPublic,
};
use sp_std::prelude::*;

use thea_primitives::{AuthorityIndex, Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Authority identifier type
		type TheaId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The maximum number of authorities that can be added.
		type MaxAuthorities: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	/// The current authorities set
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> =
		StorageValue<_, BoundedVec<T::TheaId, T::MaxAuthorities>, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, thea_primitives::ValidatorSetId, ValueQuery>;

	/// Authorities set scheduled to be used with the next session
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub(super) type NextAuthorities<T: Config> =
		StorageValue<_, BoundedVec<T::TheaId, T::MaxAuthorities>, ValueQuery>;

	/// Authority's network preference
	#[pallet::storage]
	#[pallet::getter(fn network_pref)]
	pub(super) type NetworkPreference<T: Config> =
		StorageMap<_, Identity, T::TheaId, Network, OptionQuery>;
}

impl<T: Config> Pallet<T> {
	/// Return the current active validator set.
	pub fn validator_set() -> Option<ValidatorSet<T::TheaId>> {
		let validators: BoundedVec<T::TheaId, T::MaxAuthorities> = Self::authorities();
		let id: thea_primitives::ValidatorSetId = Self::validator_set_id();
		ValidatorSet::<T::TheaId>::new(validators, id)
	}

	fn change_authorities(
		new: BoundedVec<T::TheaId, T::MaxAuthorities>,
		queued: BoundedVec<T::TheaId, T::MaxAuthorities>,
	) {
		<Authorities<T>>::put(&new);

		let new_id = Self::validator_set_id() + 1u64;
		<ValidatorSetId<T>>::put(new_id);

		<NextAuthorities<T>>::put(&queued);
	}

	fn initialize_authorities(authorities: &Vec<T::TheaId>) -> Result<(), ()> {
		if authorities.is_empty() {
			return Ok(())
		}

		if !<Authorities<T>>::get().is_empty() {
			return Err(())
		}

		let bounded_authorities =
			BoundedSlice::<T::TheaId, T::MaxAuthorities>::try_from(authorities.as_slice())
				.map_err(|_| ())?;

		let id = GENESIS_AUTHORITY_SET_ID;
		<Authorities<T>>::put(bounded_authorities);
		<ValidatorSetId<T>>::put(id);
		// Like `pallet_session`, initialize the next validator set as well.
		<NextAuthorities<T>>::put(bounded_authorities);

		Ok(())
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::TheaId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::TheaId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::TheaId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		// we panic here as runtime maintainers can simply reconfigure genesis and restart the
		// chain easily
		Self::initialize_authorities(&authorities).expect("Authorities vec too big");
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::TheaId)>,
	{
		let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		if next_authorities.len() as u32 > T::MaxAuthorities::get() {
			log::error!(
				target: "runtime::beefy",
				"authorities list {:?} truncated to length {}",
				next_authorities, T::MaxAuthorities::get(),
			);
		}
		let bounded_next_authorities =
			BoundedVec::<_, T::MaxAuthorities>::truncate_from(next_authorities);

		let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();
		if next_queued_authorities.len() as u32 > T::MaxAuthorities::get() {
			log::error!(
				target: "runtime::beefy",
				"queued authorities list {:?} truncated to length {}",
				next_queued_authorities, T::MaxAuthorities::get(),
			);
		}
		let bounded_next_queued_authorities =
			BoundedVec::<_, T::MaxAuthorities>::truncate_from(next_queued_authorities);

		// Always issue a change on each `session`, even if validator set hasn't changed.
		// We want to have at least one BEEFY mandatory block per session.
		Self::change_authorities(bounded_next_authorities, bounded_next_queued_authorities);
	}

	fn on_disabled(i: u32) {}
}

impl<T: Config> IsMember<T::TheaId> for Pallet<T> {
	fn is_member(authority_id: &T::TheaId) -> bool {
		Self::authorities().iter().any(|id| id == authority_id)
	}
}
