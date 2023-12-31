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

use crate::frost::{KeygenStages, SigningStages};
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
use thea_primitives::{
	frost::verify_params,
	types::{Message, OnChainMessage},
	Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID,
};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
mod session;
#[cfg(test)]
mod tests;

pub mod aggregator;
mod frost;
pub mod resolver;
pub mod validation;
/// Export of auto-generated weights
pub mod weights;

// TODO:
// 	 1. Detect and Slash misbehaving validator during keygen and signing
// 	 2. Redo keygen after removing misbehaving validator

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
	use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
	use thea_primitives::types::{MessageV2, ParamsForContract};

	use crate::frost::{KeygenStages, SigningStages};
	use frame_support::transactional;
	use frame_system::offchain::SendTransactionTypes;
	use thea_primitives::{
		types::{AggregatedPayload, Message, OnChainMessage},
		TheaIncomingExecutor, TheaOutgoingExecutor,
	};

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

	/// Frost KeyGen Round1
	#[pallet::storage]
	pub(super) type KeygenR1<T: Config> = StorageValue<_, BTreeMap<[u8; 32], Vec<u8>>, ValueQuery>;

	/// Frost KeyGen Round2
	#[pallet::storage]
	pub(super) type KeygenR2<T: Config> =
		StorageValue<_, BTreeMap<[u8; 32], BTreeMap<[u8; 32], Vec<u8>>>, ValueQuery>;

	/// Frost Signing Round1
	#[pallet::storage]
	pub(super) type SigningR1<T: Config> = StorageValue<_, BTreeMap<[u8; 32], Vec<u8>>, ValueQuery>;

	/// Frost Signing Round2
	#[pallet::storage]
	pub(super) type SigningR2<T: Config> =
		StorageValue<_, BTreeMap<[u8; 32], [u8; 32]>, ValueQuery>;

	/// Signed Aggregate Messages => signed params
	#[pallet::storage]
	pub(super) type SignedAggregatePayloads<T: Config> =
		StorageMap<_, Identity, u32, (AggregatedPayload, ParamsForContract), OptionQuery>;

	/// Next Aggregate Payload
	#[pallet::storage]
	pub(super) type NextAggregatePayload<T: Config> =
		StorageValue<_, BTreeSet<AggregatedPayload>, ValueQuery>;

	/// Next Aggregate Seq number
	#[pallet::storage]
	pub(super) type NextAggregateSequence<T: Config> = StorageValue<_, u32, ValueQuery>;

	/// Last Signed OutgoingNonce
	#[pallet::storage]
	pub(super) type LastSignedOutgoingNonce<T: Config> =
		StorageMap<_, Identity, Network, u64, ValueQuery>;

	/// Last Signing Stage
	#[pallet::storage]
	pub(super) type LastSigningStage<T: Config> = StorageValue<_, SigningStages, ValueQuery>;

	/// Current Frost Public key
	#[pallet::storage]
	pub(super) type CurrentTheaPublicKey<T: Config> = StorageValue<_, [u8; 65], OptionQuery>;

	/// Next Frost key voting map
	#[pallet::storage]
	pub(super) type NextTheaPublicKeyVoting<T: Config> =
		StorageValue<_, BTreeMap<[u8; 65], u16>, ValueQuery>;

	/// Next Frost Public key
	#[pallet::storage]
	pub(super) type NextTheaPublicKey<T: Config> = StorageValue<_, KeygenStages, OptionQuery>;

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
		/// Payload not found
		PayloadNotFound,
		/// Invalid Signing Stage
		InvalidSigningStage,
		/// Invalid Thea Payload
		InvalidTheaMessage,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_: BlockNumberFor<T>) -> Weight {
			let active_networks = <ActiveNetworks<T>>::get();
			let id = <ValidatorSetId<T>>::get();
			let mut pending_messages = BTreeSet::new();
			// TODO: This will happen again in every block, next_nonce message might not be used up
			// by then.
			for network in active_networks {
				let next_nonce = <LastSignedOutgoingNonce<T>>::get(network);
				match <OutgoingMessages<T>>::get(network, next_nonce) {
					None => continue,
					Some(msg) => {
						pending_messages.insert(msg.into());
					},
				}
			}
			let aggregated_payload = AggregatedPayload {
				validator_set_id: id,
				messages: pending_messages,
				is_key_change: false,
			};
			<NextAggregatePayload<T>>::mutate(|queue| {
				queue.insert(aggregated_payload);
			});
			Weight::default() // TODO: benchmark this
		}
		fn offchain_worker(blk: BlockNumberFor<T>) {
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
				Call::handle_thea_2_message { auth_index, payload, signature } =>
					Self::validate_thea_2_message(auth_index, payload, signature),
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

		/// Handles the verified thea v2 incoming messsage
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::incoming_message(1))]
		#[transactional]
		pub fn handle_thea_2_message(
			origin: OriginFor<T>,
			auth_index: u16,
			payload: OnChainMessage,
			_signature: T::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			let identifier = thea_primitives::frost::index_to_identifier(auth_index).unwrap();
			match payload {
				OnChainMessage::KR1(data) => {
					let mut map_complete = false;
					<KeygenR1<T>>::mutate(|map| {
						map.insert(identifier, data);
						if map.len() == Self::get_next_auth_max_signers() {
							map_complete = true;
						}
					});
					if map_complete {
						<NextTheaPublicKey<T>>::put(KeygenStages::R2);
					}
				},
				OnChainMessage::KR2(data) => {
					// Remove Keygen Round1 storage
					<KeygenR1<T>>::take();
					let mut map_complete = false;
					<KeygenR2<T>>::mutate(|map| {
						map.insert(identifier, data);
						if map.len() == Self::get_next_auth_max_signers() {
							map_complete = true;
						}
					});
					if map_complete {
						let id = <ValidatorSetId<T>>::get().saturating_add(1);
						<NextTheaPublicKey<T>>::put(KeygenStages::R3(id));
					}
				},
				OnChainMessage::VerifyingKey(key) => {
					<KeygenR2<T>>::take();
					// Handle majority voting and thea payload generation
					let mut got_majority = false;
					<NextTheaPublicKeyVoting<T>>::mutate(|map| {
						map.entry(key)
							.and_modify(|votes| {
								*votes = votes.saturating_add(1);
								if *votes >= Self::get_min_signers() as u16 {
									got_majority = true;
								}
							})
							.or_insert(1);
					});
					if got_majority {
						<NextTheaPublicKeyVoting<T>>::take();
						let active_networks = <ActiveNetworks<T>>::get();
						let mut messages = BTreeSet::new();
						for network in active_networks {
							let message = MessageV2 {
								nonce: 0, /* Nonce is set to zero for key change payloads, it's
								           * replay attack is handled by set id */
								data: key.to_vec(),
								network,
							};
							messages.insert(message);
						}
						let id = <ValidatorSetId<T>>::get();
						let agg_payload = AggregatedPayload {
							validator_set_id: id,
							messages,
							is_key_change: true,
						};
						<NextAggregatePayload<T>>::mutate(|queue| {
							queue.insert(agg_payload);
						});
						<NextTheaPublicKey<T>>::put(KeygenStages::Key(id.saturating_add(1), key));
					}
				},
				OnChainMessage::SR1(commitment) => {
					let mut map_complete = false;
					<SigningR1<T>>::mutate(|map| {
						map.insert(identifier, commitment);
						if map.len() == Self::get_min_signers() {
							map_complete = true;
						}
					});
					let mut payloads = <NextAggregatePayload<T>>::get();
					let agg_payload = payloads.pop_first().ok_or(Error::<T>::PayloadNotFound)?;
					if map_complete {
						<LastSigningStage<T>>::put(SigningStages::R1(agg_payload));
					}
					<NextAggregatePayload<T>>::put(payloads);
				},
				OnChainMessage::SR2(signature_share) => {
					<SigningR1<T>>::take();
					let mut map_complete = false;
					<SigningR2<T>>::mutate(|map| {
						map.insert(identifier, signature_share);
						if map.len() == Self::get_min_signers() {
							map_complete = true;
						}
					});
					if map_complete {
						let stage = <LastSigningStage<T>>::take();
						if let SigningStages::R1(payload) = stage {
							<LastSigningStage<T>>::put(SigningStages::R2(payload));
						} else {
							return Err(Error::<T>::InvalidSigningStage.into())
						}
					}
				},
				OnChainMessage::SR3(params) => {
					// Remove old data
					<SigningR2<T>>::take();
					// Store these params in storage for relayers
					let stage = <LastSigningStage<T>>::take();
					let agg_payload = if let SigningStages::R1(payload) = stage {
						<LastSigningStage<T>>::put(SigningStages::None);
						payload
					} else {
						return Err(Error::<T>::InvalidSigningStage.into())
					};
					let seq = <NextAggregateSequence<T>>::get().saturating_add(1);
					<SignedAggregatePayloads<T>>::insert(seq, (agg_payload, params));
					<NextAggregateSequence<T>>::put(seq);
				},
			}
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn get_next_auth_max_signers() -> usize {
		<NextAuthorities<T>>::get().to_vec().len()
	}
	pub fn get_min_signers() -> usize {
		let id = Self::validator_set_id();
		<Authorities<T>>::get(id)
			.to_vec()
			.len()
			.saturating_mul(2)
			.saturating_div(3)
			.saturating_add(1)
	}
	pub fn active_validators() -> Vec<T::TheaId> {
		let id = Self::validator_set_id();
		<Authorities<T>>::get(id).to_vec()
	}

	fn validate_thea_2_message(
		auth_index: &u16,
		payload: &OnChainMessage,
		signature: &<T as Config>::Signature,
	) -> TransactionValidity {
		// Check signature
		let authorities = match payload {
			OnChainMessage::KR1(_) | OnChainMessage::KR2(_) | OnChainMessage::VerifyingKey(_) =>
				<NextAuthorities<T>>::get(),
			_ => {
				let id = <ValidatorSetId<T>>::get();
				<Authorities<T>>::get(id)
			},
		};

		let encoded_payload = payload.encode();
		let msg_hash = sp_io::hashing::blake2_256(&encoded_payload);
		match authorities.get(*auth_index as usize) {
			None => return InvalidTransaction::Custom(1).into(),
			Some(auth) =>
				if !auth.verify(&msg_hash, &((*signature).clone().into())) {
					return InvalidTransaction::Custom(2).into()
				},
		}

		// Now we need to verify only verifying share and final params
		match payload {
			OnChainMessage::VerifyingKey(pub_key) => match <NextTheaPublicKey<T>>::get() {
				None => return InvalidTransaction::Custom(3).into(),
				Some(stage) => match stage {
					KeygenStages::R3(id) => {},
					_ => return InvalidTransaction::Custom(3).into(),
				},
			},
			OnChainMessage::SR3(params) => {
				match <LastSigningStage<T>>::get() {
					SigningStages::R2(_) => {},
					_ => return InvalidTransaction::Custom(3).into(),
				}

				match <CurrentTheaPublicKey<T>>::get() {
					None => return InvalidTransaction::Custom(5).into(),
					Some(verifying_key) => {
						// Verify params with verifying key
						if !verify_params(verifying_key, params) {
							return InvalidTransaction::Custom(6).into();
						}
					},
				}
			},
			OnChainMessage::KR1(data) => match <NextTheaPublicKey<T>>::get() {
				None => return InvalidTransaction::Custom(3).into(),
				Some(stage) => match stage {
					KeygenStages::R1 => {
						if !frost_secp256k1::keys::dkg::round1::Package::deserialize(&data).is_ok()
						{
							// TODO: Slash this authority
							return InvalidTransaction::Custom(4).into()
						}
					},
					_ => return InvalidTransaction::Custom(3).into(),
				},
			},
			OnChainMessage::KR2(data_map) => match <NextTheaPublicKey<T>>::get() {
				None => return InvalidTransaction::Custom(3).into(),
				Some(stage) => match stage {
					KeygenStages::R2 => {
						for (_, v) in data_map {
							if !frost_secp256k1::keys::dkg::round2::Package::deserialize(v).is_ok()
							{
								// TODO: Slash this authority
								return InvalidTransaction::Custom(4).into()
							}
						}
					},
					_ => return InvalidTransaction::Custom(3).into(),
				},
			},
			OnChainMessage::SR1(data) => match <LastSigningStage<T>>::get() {
				SigningStages::None => {
					if !frost_secp256k1::round1::SigningCommitments::deserialize(data).is_ok() {
						// TODO: Slash this authority
						return InvalidTransaction::Custom(4).into()
					}
				},
				_ => return InvalidTransaction::Custom(3).into(),
			},
			OnChainMessage::SR2(_) => match <LastSigningStage<T>>::get() {
				SigningStages::R1(_) => {},
				_ => return InvalidTransaction::Custom(3).into(),
			},
		}

		ValidTransaction::with_tag_prefix("thea")
			.and_provides(payload)
			.longevity(1)
			.propagate(true)
			.build()
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
			<NextTheaPublicKey<T>>::put(KeygenStages::R1);
			// Queued set will do keygen and send the new public key to other ecosystems.
			// This should happen at the beginning of the last epoch
			if let Some(validator_set) = ValidatorSet::new(queued.clone(), new_id) {
				let payload = validator_set.encode();
				// TODO: Instead of generating the same payload for all active networks,
				// just sign one payload and send it to everyone - Done in Thea v2
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
			match <NextTheaPublicKey<T>>::get() {
				None => {
					log::error!(target:"thea","This should never happen, next public key should be available at this time");
				},
				Some(key_stage) => match key_stage {
					KeygenStages::Key(expected_new_id, key) =>
						if new_id == expected_new_id {
							<CurrentTheaPublicKey<T>>::put(key)
						} else {
							log::error!(target:"thea","This should never happen, validator set id should be correct");
						},
					_ => {
						log::error!(target:"thea","This should never happen, next public key should be available at this time");
					},
				},
			}
			// New public key takes effect
			// This will happen when new era starts, or end of the last epoch
			<Authorities<T>>::insert(new_id, incoming);
			<ValidatorSetId<T>>::put(new_id);
			// TODO: Remove the below code block once Thea V2 is active
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
