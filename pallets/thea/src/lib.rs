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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

//! # Thea pallet.
//!
//! The core logic for runtime is handled by the Thea pallet. The most important
//! responsibilities of the Thea pallet are to:
//! * process ingress messages to the runtime;
//! * keep track of egress messages;
//! * handle validator session changes;

use frame_support::{pallet_prelude::*, traits::Get, BoundedVec, Parameter};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use parity_scale_codec::Encode;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	traits::{BlockNumberProvider, Member},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	Percent, RuntimeAppPublic, SaturatedConversion,
};
use sp_std::prelude::*;
use thea_primitives::{types::Message, Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
mod session;
#[cfg(test)]
mod tests;

pub mod validation;
/// Export of auto-generated weights
pub mod weights;

pub const THEA: KeyTypeId = KeyTypeId(*b"thea");

pub mod ecdsa {
	mod app_ecdsa {
		use sp_application_crypto::{app_crypto, ecdsa};

		use super::super::THEA;

		app_crypto!(ecdsa, THEA);
	}

	sp_application_crypto::with_pair! {
		/// An THEA keypair using ecdsa as its crypto.
		pub type AuthorityPair = app_ecdsa::Pair;
	}

	/// An THEA signature using ecdsa as its crypto.
	pub type AuthoritySignature = app_ecdsa::Signature;

	/// An THEA identifier using ecdsa as its crypto.
	pub type AuthorityId = app_ecdsa::Public;
}

pub trait TheaWeightInfo {
	fn incoming_message(b: u32) -> Weight;
	fn send_thea_message(_b: u32) -> Weight;
	fn update_incoming_nonce(_b: u32) -> Weight;
	fn update_outgoing_nonce(_b: u32) -> Weight;
	fn add_thea_network() -> Weight;
	fn remove_thea_network() -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::transactional;
	use frame_system::offchain::SendTransactionTypes;
	use sp_std::collections::btree_set::BTreeSet;
	use thea_primitives::{types::Message, TheaIncomingExecutor, TheaOutgoingExecutor};

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Authority identifier type
		type TheaId: Member + Parameter + RuntimeAppPublic + MaybeSerializeDeserialize + Ord;

		/// Authority Signature
		type Signature: IsType<<Self::TheaId as RuntimeAppPublic>::Signature> + Member + Parameter;

		/// The maximum number of authorities that can be added.
		type MaxAuthorities: Get<u32>;

		/// Something that executes the payload
		type Executor: thea_primitives::TheaIncomingExecutor;

		/// Type representing the weight of this pallet
		type WeightInfo: TheaWeightInfo;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	/// The current authorities set
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> = StorageMap<
		_,
		Identity,
		thea_primitives::ValidatorSetId,
		BoundedVec<T::TheaId, T::MaxAuthorities>,
		ValueQuery,
	>;

	/// The current validator set id, it will increment by 1 on every epoch.
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, thea_primitives::ValidatorSetId, ValueQuery>;

	/// Authorities set scheduled to be used with the next session
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub(super) type NextAuthorities<T: Config> =
		StorageValue<_, BoundedVec<T::TheaId, T::MaxAuthorities>, ValueQuery>;

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

	/// List of Active networks
	#[pallet::storage]
	#[pallet::getter(fn active_networks)]
	pub(super) type ActiveNetworks<T: Config> = StorageValue<_, BTreeSet<Network>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		TheaPayloadProcessed(Network, u64),
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

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn offchain_worker(blk: T::BlockNumber) {
			log::debug!(target:"thea","Thea offchain worker started");
			if let Err(err) = Self::run_thea_validation(blk) {
				log::error!(target:"thea","Error while running thea: {:?}",err);
			}
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::incoming_message { payload, signatures } =>
					Self::validate_incoming_message(payload, signatures),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Handles the verified incoming message
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::incoming_message(1))]
		#[transactional]
		pub fn incoming_message(
			origin: OriginFor<T>,
			payload: Message,
			_signatures: Vec<(u16, T::Signature)>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// Signature and nonce are already verified in validate_unsigned, no need to do it again
			T::Executor::execute_deposits(payload.network, payload.data.clone());
			<IncomingNonce<T>>::insert(payload.network, payload.nonce);
			Self::deposit_event(Event::<T>::TheaPayloadProcessed(payload.network, payload.nonce));
			// Save the incoming message for some time
			<IncomingMessages<T>>::insert(payload.network, payload.nonce, payload);
			Ok(())
		}

		/// Send some arbitrary data to the given network
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::send_thea_message(1))]
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
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::update_incoming_nonce(1))]
		#[transactional]
		pub fn update_incoming_nonce(
			origin: OriginFor<T>,
			nonce: u64,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			<IncomingNonce<T>>::insert(network, nonce);
			Ok(())
		}

		/// A governance endpoint to update last processed nonce
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::update_outgoing_nonce(1))]
		#[transactional]
		pub fn update_outgoing_nonce(
			origin: OriginFor<T>,
			nonce: u64,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			<OutgoingNonce<T>>::insert(network, nonce);
			Ok(())
		}

		/// Add a network to active networks
		#[pallet::call_index(4)]
		#[pallet::weight(< T as Config >::WeightInfo::add_thea_network())]
		pub fn add_thea_network(origin: OriginFor<T>, network: Network) -> DispatchResult {
			ensure_root(origin)?;
			<ActiveNetworks<T>>::mutate(|list| {
				list.insert(network);
			});
			Ok(())
		}

		/// Remove a network to active networks
		#[pallet::call_index(5)]
		#[pallet::weight(< T as Config >::WeightInfo::remove_thea_network())]
		pub fn remove_thea_network(origin: OriginFor<T>, network: Network) -> DispatchResult {
			ensure_root(origin)?;
			<ActiveNetworks<T>>::mutate(|list| {
				list.remove(&network);
			});
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn active_validators() -> Vec<T::TheaId> {
		let id = Self::validator_set_id();
		<Authorities<T>>::get(id).to_vec()
	}

	fn validate_incoming_message(
		payload: &Message,
		signatures: &Vec<(u16, T::Signature)>,
	) -> TransactionValidity {
		// Check if this message can be processed next by checking its nonce
		let next_nonce = <IncomingNonce<T>>::get(payload.network).saturating_add(1);

		if payload.nonce != next_nonce {
			return InvalidTransaction::Custom(1).into()
		}

		// Incoming messages are always signed by the current validators.
		let current_set_id = <ValidatorSetId<T>>::get();
		let authorities = <Authorities<T>>::get(current_set_id).to_vec();

		// Check for super majority
		const MAJORITY: u8 = 67;
		let p = Percent::from_percent(MAJORITY);
		let threshold = p * authorities.len();

		if signatures.len() < threshold {
			return InvalidTransaction::Custom(4).into()
		}

		let encoded_payload = payload.encode();
		let msg_hash = sp_io::hashing::sha2_256(&encoded_payload);
		for (index, signature) in signatures {
			match authorities.get(*index as usize) {
				None => return InvalidTransaction::Custom(2).into(),
				Some(auth) =>
					if !auth.verify(&msg_hash, &((*signature).clone().into())) {
						return InvalidTransaction::Custom(3).into()
					},
			}
		}

		ValidTransaction::with_tag_prefix("thea")
			.and_provides(payload)
			.longevity(3)
			.propagate(true)
			.build()
	}

	pub fn generate_payload(is_key_change: bool, network: Network, data: Vec<u8>) -> Message {
		// Generate the Thea payload to communicate with foreign chains
		let nonce = <OutgoingNonce<T>>::get(network);
		let id = Self::validator_set_id();
		Message {
			block_no: frame_system::Pallet::<T>::current_block_number().saturated_into(),
			nonce: nonce.saturating_add(1),
			data,
			network,
			is_key_change,
			validator_set_id: id,
		}
	}

	fn change_authorities(
		incoming: BoundedVec<T::TheaId, T::MaxAuthorities>, // n+1th set
		queued: BoundedVec<T::TheaId, T::MaxAuthorities>,   // n+ 2th set
	) {
		//	( outgoing) -> (validators/incoming) -> (queued)
		// nth epoch -> n+1th epoch -> n+2nd epoch
		let id = Self::validator_set_id();
		let outgoing = <Authorities<T>>::get(id); // nth set  ( active ,current )
		let new_id = id + 1u64;
		let active_networks = <ActiveNetworks<T>>::get();
		// We need to issue a new message if the validator set is changing,
		// that is, the incoming set is has different session keys from outgoing set.
		// This last message should be signed by the outgoing set
		// Similar to how Grandpa's session change works.
		if incoming != queued {
			// This should happen at the beginning of the last epoch
			if let Some(validator_set) = ValidatorSet::new(queued.clone(), new_id) {
				let payload = validator_set.encode();
				for network in &active_networks {
					let message = Self::generate_payload(true, *network, payload.clone());
					// Update nonce
					<OutgoingNonce<T>>::insert(message.network, message.nonce);
					<OutgoingMessages<T>>::insert(message.network, message.nonce, message);
				}
			}
			<NextAuthorities<T>>::put(queued);
		}
		if incoming != outgoing {
			// This will happen when new era starts, or end of the last epoch
			<Authorities<T>>::insert(new_id, incoming);
			<ValidatorSetId<T>>::put(new_id);
			for network in active_networks {
				let message = Self::generate_payload(false, network, Vec::new());
				<OutgoingNonce<T>>::insert(network, message.nonce);
				<OutgoingMessages<T>>::insert(network, message.nonce, message);
			}
		}
	}

	fn initialize_authorities(authorities: &[T::TheaId]) -> Result<(), ()> {
		let id = GENESIS_AUTHORITY_SET_ID;
		<ValidatorSetId<T>>::put(id);
		<Authorities<T>>::insert(id, BoundedVec::truncate_from(authorities.to_vec()));
		Ok(())
	}

	pub fn get_outgoing_messages(network: Network, nonce: u64) -> Option<Message> {
		<OutgoingMessages<T>>::get(network, nonce)
	}

	pub fn get_last_processed_nonce(network: Network) -> u64 {
		<IncomingNonce<T>>::get(network)
	}
}

impl<T: Config> thea_primitives::TheaOutgoingExecutor for Pallet<T> {
	fn execute_withdrawals(network: Network, data: Vec<u8>) -> DispatchResult {
		let payload = Self::generate_payload(false, network, data);
		// Update nonce
		<OutgoingNonce<T>>::insert(network, payload.nonce);
		<OutgoingMessages<T>>::insert(network, payload.nonce, payload);
		Ok(())
	}
}
