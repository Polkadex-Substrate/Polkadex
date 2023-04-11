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

use frame_support::{
	log,
	pallet_prelude::*,
	traits::{Get, OneSessionHandler},
	BoundedSlice, BoundedVec, Parameter,
};
use frame_system::pallet_prelude::*;
use parity_scale_codec::{Encode, MaxEncodedLen};
use sp_runtime::{
	generic::DigestItem,
	traits::{BlockNumberProvider, IsMember, Member},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeAppPublic, SaturatedConversion,
};
use sp_std::prelude::*;

pub use pallet::*;
use thea_primitives::{
	types::Message, AuthorityIndex, Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID,
};

mod session;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::transactional;
	use thea_primitives::{types::Message, TheaIncomingExecutor};

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Authority identifier type
		type TheaId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// Authority Signature
		type Signature: IsType<<Self::TheaId as RuntimeAppPublic>::Signature>
			+ Member
			+ Parameter
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// The maximum number of authorities that can be added.
		type MaxAuthorities: Get<u32>;

		/// Something that executes the payload
		type Executor: thea_primitives::TheaIncomingExecutor;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
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

	/// Outgoing messages
	/// first key: Blocknumber of polkadex solochain
	/// second key: receiving network
	#[pallet::storage]
	#[pallet::getter(fn outgoing_messages)]
	pub(super) type OutgoingMessages<T: Config> =
		StorageDoubleMap<_, Identity, T::BlockNumber, Identity, Network, Message, OptionQuery>;

	/// Incoming messages
	/// first key: origin network
	/// second key: origin network blocknumber
	#[pallet::storage]
	#[pallet::getter(fn incoming_messages)]
	pub(super) type IncomingMessages<T: Config> =
		StorageDoubleMap<_, Identity, Network, Identity, T::BlockNumber, Message, OptionQuery>;

	/// Last processed blocks of other networks
	#[pallet::storage]
	#[pallet::getter(fn last_processed_blk)]
	pub(super) type LastProcessedBlock<T: Config> =
		StorageMap<_, Identity, Network, T::BlockNumber, OptionQuery>;

	/// Last processed nonce of other networks
	#[pallet::storage]
	#[pallet::getter(fn last_processed_nonce)]
	pub(super) type LastProcessedNonce<T: Config> =
		StorageMap<_, Identity, Network, u64, ValueQuery>;

	/// Outgoing nonce's grouped by network
	#[pallet::storage]
	#[pallet::getter(fn outgoing_nonce)]
	pub(super) type OutgoingNonce<T: Config> = StorageMap<_, Identity, Network, u64, ValueQuery>;

	// /// Last processed message from Polkadex
	// #[pallet::storage]
	// #[pallet::getter(fn last_processed_polkadex_blk)]
	// pub(super) type LastProcessedPolkadexBlk<T: Config> = StorageValue<_, u64, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		NetworkUpdated { authority: T::TheaId, network: Network },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Error
		Unknown,
		/// Error executing thea message
		ErrorExecutingMessage,
		/// Wrong nonce provided
		MessageNonce,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::update_network_pref { authority, network, signature } =>
					Self::validate_update_network_pref(authority, network, signature),
				Call::incoming_message { bitmap, payload, signature } =>
					Self::validate_incoming_message(bitmap, payload, signature),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Updates the network preference of a thea validator
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn update_network_pref(
			origin: OriginFor<T>,
			authority: T::TheaId,
			network: Network,
			_signature: T::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			<NetworkPreference<T>>::insert(authority, network);
			Ok(())
		}

		/// Handles the verified incoming message
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		#[transactional]
		pub fn incoming_message(
			origin: OriginFor<T>,
			_bitmap: Vec<u128>,
			payload: Message,
			_signature: T::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			// Signature is already verified in validate_unsigned, no need to do it again

			let last_nonce = <LastProcessedNonce<T>>::get(payload.network);
			if last_nonce != payload.nonce.saturating_add(1) {
				return Err(Error::<T>::MessageNonce.into())
			}

			if let Err(()) = T::Executor::execute_deposits(payload.network, payload.data.clone()) {
				return Err(Error::<T>::ErrorExecutingMessage.into())
			}

			<LastProcessedNonce<T>>::insert(payload.network, payload.nonce);
			<LastProcessedBlock<T>>::insert(
				payload.network,
				payload.block_no.saturated_into::<T::BlockNumber>(),
			);
			// Save the incoming message for some time
			<IncomingMessages<T>>::insert(
				payload.network,
				payload.block_no.saturated_into::<T::BlockNumber>(),
				payload,
			);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn active_validators() -> Vec<T::TheaId> {
		<Authorities<T>>::get().to_vec()
	}

	pub fn next_validators() -> Vec<T::TheaId> {
		<NextAuthorities<T>>::get().to_vec()
	}

	pub fn authority_network_pref(authority: &T::TheaId) -> Option<Network> {
		<NetworkPreference<T>>::get(authority)
	}

	fn validate_update_network_pref(
		authority: &T::TheaId,
		network: &Network,
		signature: &T::Signature,
	) -> TransactionValidity {
		let queued = <NextAuthorities<T>>::get();
		// They should be part of next authorities
		if !queued.contains(authority) {
			return InvalidTransaction::BadSigner.into()
		}
		// verify signature
		if !authority.verify(&network.encode(), &signature.clone().into()) {
			return InvalidTransaction::BadSigner.into()
		}

		ValidTransaction::with_tag_prefix("thea")
			.and_provides([authority])
			.longevity(3)
			.propagate(true)
			.build()
	}

	fn validate_incoming_message(
		bitmap: &Vec<u128>,
		payload: &Message,
		signature: &T::Signature,
	) -> TransactionValidity {
		// TODO: Implement aggregate signature verification using bls_primitives library.

		ValidTransaction::with_tag_prefix("thea")
			.and_provides([signature])
			.longevity(3)
			.propagate(true)
			.build()
	}

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

impl<T: Config> thea_primitives::TheaOutgoingExecutor for Pallet<T> {
	fn execute_withdrawals(network: Network, data: Vec<u8>) -> Result<(), ()> {
		let nonce = <OutgoingNonce<T>>::get(network);
		let payload = Message {
			block_no: frame_system::Pallet::<T>::current_block_number().saturated_into(),
			nonce: nonce.saturating_add(1),
			data,
			network,
		};
		// Update nonce
		<OutgoingNonce<T>>::insert(network, payload.nonce);
		<OutgoingMessages<T>>::insert(
			payload.block_no.saturated_into::<T::BlockNumber>(),
			payload.network,
			payload,
		);
		Ok(())
	}
}
