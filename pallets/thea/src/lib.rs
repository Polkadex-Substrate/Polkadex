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

use frame_support::{pallet_prelude::*, traits::Get, BoundedVec, Parameter};
use frame_system::{offchain::SubmitTransaction, pallet_prelude::*};
use parity_scale_codec::{Encode, MaxEncodedLen};
use sp_runtime::{
	traits::{BlockNumberProvider, Member},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	RuntimeAppPublic, SaturatedConversion,
};
use sp_std::prelude::*;

pub use pallet::*;
use polkadex_primitives::utils::return_set_bits;
use thea_primitives::{
	types::Message, Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID, NATIVE_NETWORK,
};

mod session;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::transactional;
	use frame_system::offchain::SendTransactionTypes;

	use thea_primitives::{types::Message, TheaIncomingExecutor, TheaOutgoingExecutor};

	use super::*;

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Authority identifier type
		type TheaId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ Into<bls_primitives::Public>;

		/// Authority Signature
		type Signature: IsType<<Self::TheaId as RuntimeAppPublic>::Signature>
			+ Member
			+ Parameter
			+ Into<bls_primitives::Signature>;

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
		StorageMap<_, Identity, Network, BoundedVec<T::TheaId, T::MaxAuthorities>, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, thea_primitives::ValidatorSetId, ValueQuery>;

	/// Authorities set scheduled to be used with the next session
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub(super) type NextAuthorities<T: Config> =
		StorageMap<_, Identity, Network, BoundedVec<T::TheaId, T::MaxAuthorities>, ValueQuery>;

	/// Authority's network preference
	#[pallet::storage]
	#[pallet::getter(fn network_pref)]
	pub(super) type NetworkPreference<T: Config> =
		StorageMap<_, Identity, T::TheaId, Network, OptionQuery>;

	/// Outgoing messages
	/// first key: Network
	/// second key: Message nonce
	#[pallet::storage]
	#[pallet::getter(fn outgoing_messages)]
	pub(super) type OutgoingMessages<T: Config> =
		StorageDoubleMap<_, Identity, Network, Identity, u64, Message, OptionQuery>;

	/// Incoming messages
	/// first key: origin network
	/// second key: origin network blocknumber
	#[pallet::storage]
	#[pallet::getter(fn incoming_messages)]
	pub(super) type IncomingMessages<T: Config> =
		StorageDoubleMap<_, Identity, Network, Identity, u64, Message, OptionQuery>;

	/// Last processed nonce of other networks
	#[pallet::storage]
	#[pallet::getter(fn last_processed_nonce)]
	pub(super) type IncomingNonce<T: Config> = StorageMap<_, Identity, Network, u64, ValueQuery>;

	/// Outgoing nonce's grouped by network
	#[pallet::storage]
	#[pallet::getter(fn outgoing_nonce)]
	pub(super) type OutgoingNonce<T: Config> = StorageMap<_, Identity, Network, u64, ValueQuery>;

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
		/// No validators for this network
		NoValidatorsFound(Network),
		/// Cannot update with older nonce
		NonceIsAlreadyProcessed,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
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
			<NetworkPreference<T>>::insert(authority.clone(), network);
			Self::deposit_event(Event::NetworkUpdated { authority, network });
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

			let last_nonce = <IncomingNonce<T>>::get(payload.network);
			if last_nonce.saturating_add(1) != payload.nonce {
				return Err(Error::<T>::MessageNonce.into())
			}
			T::Executor::execute_deposits(payload.network, payload.data.clone());
			<IncomingNonce<T>>::insert(payload.network, payload.nonce);
			// Save the incoming message for some time
			<IncomingMessages<T>>::insert(payload.network, payload.nonce, payload);
			Ok(())
		}

		/// Send some arbitary data to the given network
		#[pallet::call_index(2)]
		#[pallet::weight(Weight::default())]
		#[transactional]
		pub fn send_thea_message(
			origin: OriginFor<T>,
			data: Vec<u8>,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::execute_withdrawals(network, data)?;
			Ok(())
		}

		/// A governance endpoint to update last processed nonce
		#[pallet::call_index(3)]
		#[pallet::weight(Weight::default())]
		#[transactional]
		pub fn update_incoming_nonce(
			origin: OriginFor<T>,
			nonce: u64,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			let last_nonce = <IncomingNonce<T>>::get(network);
			// Nonce can only be changed forwards, already processed nonces should not be changed.
			if last_nonce >= nonce {
				return Err(Error::<T>::NonceIsAlreadyProcessed.into())
			}
			<IncomingNonce<T>>::insert(network, nonce);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn active_validators(network: Network) -> Vec<T::TheaId> {
		<Authorities<T>>::get(network).to_vec()
	}

	pub fn authority_network_pref(authority: &T::TheaId) -> Option<Network> {
		<NetworkPreference<T>>::get(authority)
	}

	fn validate_incoming_message(
		bitmap: &[u128],
		payload: &Message,
		signature: &T::Signature,
	) -> TransactionValidity {
		// Check if this message can be processed next by checking its nonce
		let nonce = <IncomingNonce<T>>::get(payload.network);
		if payload.nonce != nonce.saturating_add(1) {
			return Err(InvalidTransaction::Custom(1).into())
		}

		// Find who all signed this payload
		let signed_auths_indexes: Vec<usize> = return_set_bits(bitmap);
		// TODO: Check if we have 2/3rd authorities signed on this.
		// TODO: Make, <Authorities<T>> indexed by network as key1 and validator setid as key2
		// Create a vector of public keys of everyone who signed
		let auths: Vec<T::TheaId> = <Authorities<T>>::get(payload.network).to_vec();
		let mut signatories: Vec<bls_primitives::Public> = vec![];
		for index in signed_auths_indexes {
			match auths.get(index) {
				None => return Err(InvalidTransaction::Custom(2).into()),
				Some(auth) => signatories.push(auth.clone().into()),
			}
		}
		// Verify the aggregate signature.
		let bls_signature: bls_primitives::Signature = signature.clone().into();
		if !bls_signature.verify(&signatories, payload.encode().as_ref()) {
			return Err(InvalidTransaction::BadSigner.into())
		}

		ValidTransaction::with_tag_prefix("thea")
			.and_provides([signature])
			.longevity(3)
			.propagate(true)
			.build()
	}

	/// Return the current active validator set for all networks
	pub fn full_validator_set() -> Option<ValidatorSet<T::TheaId>> {
		let mut full_list = sp_std::vec::Vec::new();
		for list in <Authorities<T>>::iter_values() {
			full_list.append(&mut list.to_vec())
		}
		let id: thea_primitives::ValidatorSetId = Self::validator_set_id();
		ValidatorSet::<T::TheaId>::new(full_list, id)
	}

	/// Return the current active validator set.
	pub fn validator_set(network: Network) -> Option<ValidatorSet<T::TheaId>> {
		let validators: BoundedVec<T::TheaId, T::MaxAuthorities> = Self::authorities(network);
		let id: thea_primitives::ValidatorSetId = Self::validator_set_id();
		ValidatorSet::<T::TheaId>::new(validators, id)
	}

	fn change_authorities(
		new: BoundedVec<T::TheaId, T::MaxAuthorities>,
		queued: BoundedVec<T::TheaId, T::MaxAuthorities>,
	) {
		if new == queued {
			// Don't do anything if there is not change in new and queued validators
			return
		}
		let group_by = |list: &BoundedVec<T::TheaId, T::MaxAuthorities>| -> sp_std::collections::btree_map::BTreeMap<
			Network,
			BoundedVec<T::TheaId, T::MaxAuthorities>,
		> {
			let mut map = sp_std::collections::btree_map::BTreeMap::new();
			for auth in list {
				if let Some(network) = <NetworkPreference<T>>::get(auth) {
					map.entry(network)
						.and_modify(|list: &mut BoundedVec<T::TheaId, T::MaxAuthorities>| {
							// Force push is fine as the subset of network will be less than
							// or equal to max validators
							list.force_push(auth.clone());
						})
						.or_insert(BoundedVec::truncate_from(sp_std::vec::Vec::from([
							auth.clone()
						])));
				} else {
					// TODO: Make it an offence to not provide network as part of next version
				}
			}
			map
		};

		for (network, list) in &group_by(&new) {
			<Authorities<T>>::insert(network, list);
		}

		let new_id = Self::validator_set_id() + 1u64;
		<ValidatorSetId<T>>::put(new_id);

		for (network, list) in &group_by(&queued) {
			// Store the queued authorities
			<NextAuthorities<T>>::insert(network, list);
			// Generate the Thea payload to communicate with foreign chains
			let nonce = <OutgoingNonce<T>>::get(network);
			let payload = Message {
				block_no: frame_system::Pallet::<T>::current_block_number().saturated_into(),
				nonce: nonce.saturating_add(1),
				data: list.encode(),
				network: NATIVE_NETWORK,
				is_key_change: true,
				validator_set_id: Self::validator_set_id(),
				validator_set_len: Self::authorities(network).len().saturated_into(),
			};
			// Update nonce
			<OutgoingNonce<T>>::insert(network, payload.nonce);
			<OutgoingMessages<T>>::insert(payload.network, payload.nonce, payload);
		}
	}

	fn initialize_authorities(authorities: &[T::TheaId]) -> Result<(), ()> {
		let id = GENESIS_AUTHORITY_SET_ID;
		<ValidatorSetId<T>>::put(id);

		<Authorities<T>>::insert(1, BoundedVec::truncate_from(authorities.to_vec()));
		for auth in authorities {
			// Everyone is assigned to one on genesis.
			<NetworkPreference<T>>::insert(auth.clone(), 1);
		}
		Ok(())
	}

	pub fn get_outgoing_messages(network: Network, nonce: u64) -> Option<Message> {
		<OutgoingMessages<T>>::get(network, nonce)
	}

	pub fn network(auth: T::TheaId) -> Option<Network> {
		<NetworkPreference<T>>::get(auth)
	}

	#[allow(clippy::result_unit_err)]
	pub fn submit_incoming_message(
		payload: Message,
		bitmap: Vec<u128>,
		signature: T::Signature,
	) -> Result<(), ()> {
		let call = Call::<T>::incoming_message { bitmap, payload, signature };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
	}

	pub fn get_last_processed_nonce(network: Network) -> u64 {
		<IncomingNonce<T>>::get(network)
	}
}

impl<T: Config> thea_primitives::TheaOutgoingExecutor for Pallet<T> {
	fn execute_withdrawals(network: Network, data: Vec<u8>) -> DispatchResult {
		let auth_len = Self::authorities(network).len();
		if auth_len == 0 {
			return Err(Error::<T>::NoValidatorsFound(network).into())
		}
		let nonce = <OutgoingNonce<T>>::get(network);
		let payload = Message {
			block_no: frame_system::Pallet::<T>::current_block_number().saturated_into(),
			nonce: nonce.saturating_add(1),
			data,
			network: NATIVE_NETWORK,
			is_key_change: false,
			validator_set_id: Self::validator_set_id(),
			validator_set_len: auth_len.saturated_into(),
		};
		// Update nonce
		<OutgoingNonce<T>>::insert(network, payload.nonce);
		<OutgoingMessages<T>>::insert(network, payload.nonce, payload);
		Ok(())
	}
}
