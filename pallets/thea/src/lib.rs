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
	RuntimeAppPublic, SaturatedConversion,
};
use sp_std::prelude::*;
use thea_primitives::{
	types::{Message, NetworkType, PayloadType},
	Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID,
};

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
	fn submit_incoming_message(b: u32) -> Weight;
	fn send_thea_message(_b: u32) -> Weight;
	fn update_incoming_nonce(_b: u32) -> Weight;
	fn update_outgoing_nonce(_b: u32) -> Weight;
	fn add_thea_network() -> Weight;
	fn remove_thea_network() -> Weight;
	fn submit_signed_outgoing_messages() -> Weight;
	fn report_misbehaviour() -> Weight;
	fn handle_misbehaviour() -> Weight;
	fn on_initialize(x: u32) -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		traits::{
			fungible::{Inspect, Mutate as OtherMutate},
			tokens::{fungible::hold::Mutate, Fortitude, Precision, Preservation},
		},
		transactional,
	};
	use frame_system::offchain::SendTransactionTypes;
	use polkadex_primitives::Balance;
	use sp_std::collections::btree_set::BTreeSet;
	use thea_primitives::{
		types::{IncomingMessage, Message, MisbehaviourReport, SignedMessage, THEA_HOLD_REASON},
		TheaIncomingExecutor, TheaOutgoingExecutor,
	};

	#[pallet::config]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>>{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Authority identifier type
		type TheaId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ Ord
			+ Into<sp_core::ecdsa::Public>
			+ From<sp_core::ecdsa::Public>;

		/// Authority Signature
		type Signature: IsType<<Self::TheaId as RuntimeAppPublic>::Signature>
			+ Member
			+ Parameter
			+ From<sp_core::ecdsa::Signature>
			+ Into<sp_core::ecdsa::Signature>;

		/// The maximum number of authorities that can be added.
		type MaxAuthorities: Get<u32>;

		/// Something that executes the payload
		type Executor: thea_primitives::TheaIncomingExecutor;

		/// Balances Pallet
		type NativeCurrency: frame_support::traits::fungible::Mutate<Self::AccountId>
			+ frame_support::traits::fungible::Inspect<Self::AccountId>
			+ frame_support::traits::fungible::hold::Mutate<Self::AccountId, Reason = [u8; 8]>;

		/// Governance Origin
		type TheaGovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		#[cfg(feature = "runtime-benchmarks")]
		type TheaBenchmarkHelper: thea_primitives::TheaBenchmarkHelper;

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

	/// Signed Outgoing messages
	/// first key: Network
	/// second key: Message nonce
	#[pallet::storage]
	pub(super) type SignedOutgoingMessages<T: Config> = StorageDoubleMap<
		_,
		Identity,
		Network,
		Identity,
		u64,
		SignedMessage<T::Signature>,
		OptionQuery,
	>;

	/// Incoming messages queue
	/// first key: origin network
	/// second key: blocknumber at which it will execute
	#[pallet::storage]
	pub(super) type IncomingMessagesQueue<T: Config> = StorageDoubleMap<
		_,
		Identity,
		Network,
		Identity,
		u64,
		thea_primitives::types::IncomingMessage<T::AccountId, Balance>,
		OptionQuery,
	>;

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

	/// Outgoing signed nonce's grouped by network
	#[pallet::storage]
	pub(super) type SignedOutgoingNonce<T: Config> =
		StorageMap<_, Identity, Network, u64, ValueQuery>;

	/// List of Active networks
	#[pallet::storage]
	#[pallet::getter(fn active_networks)]
	pub(super) type ActiveNetworks<T: Config> = StorageValue<_, BTreeSet<Network>, ValueQuery>;

	/// Network Config
	#[pallet::storage]
	pub(super) type NetworkConfig<T: Config> =
		StorageMap<_, Identity, Network, thea_primitives::types::NetworkConfig, ValueQuery>;

	/// Misbehavour Reports
	/// first key: origin network
	/// second key: nonce
	#[pallet::storage]
	pub(super) type MisbehaviourReports<T: Config> = StorageDoubleMap<
		_,
		Identity,
		Network,
		Identity,
		u64,
		thea_primitives::types::MisbehaviourReport<T::AccountId, Balance>,
		OptionQuery,
	>;

	/// Temporary allowlist for relayer
	#[pallet::storage]
	pub(super) type AllowListTestingRelayers<T: Config> =
		StorageMap<_, Identity, Network, T::AccountId, OptionQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		TheaPayloadProcessed(Network, u64),
		ErrorWhileReleasingLock(T::AccountId, DispatchError),
		/// Misbehaviour Reported (fisherman, network, nonce)
		MisbehaviourReported(T::AccountId, Network, u64),
		/// New signature of Thea withdrawal
		TheaSignatureUpdated(Network, u64, u16),
		/// Signing completed
		TheaSignatureFinalized(Network, u64),
		/// Unable to parse public key
		UnableToParsePublicKey(T::TheaId),
		/// Unable to slice public key hash for evm chains
		UnableToSlicePublicKeyHash(T::TheaId),
		/// Unable to generate rotate validators payload for this network
		UnableToGenerateValidatorSet(Network),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Error
		Unknown,
		/// Error executing thea message
		ErrorExecutingMessage,
		/// Wrong nonce provided
		MessageNonce,
		/// Not enough stake
		NotEnoughStake,
		/// MessageNotFound
		MessageNotFound,
		/// No Relayer found
		NoRelayersFound,
		/// Not expected relayer origin
		NotAnAllowlistedRelayer,
		/// Nonce Error
		NonceError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(blk: BlockNumberFor<T>) -> Weight {
			// Every block check the next incoming nonce and if fork period is over, execute them
			let active_networks = <ActiveNetworks<T>>::get();
			for network in active_networks.clone() {
				let last_processed_nonce = <IncomingNonce<T>>::get(network);
				let next_nonce = last_processed_nonce.saturating_add(1);
				match <IncomingMessagesQueue<T>>::take(network, next_nonce) {
					None => continue,
					Some(msg) => {
						if msg.execute_at <= blk.saturated_into::<u32>() {
							<T as pallet::Config>::Executor::execute_deposits(
								msg.message.network,
								msg.message.data.clone(),
							);
							<IncomingNonce<T>>::insert(msg.message.network, next_nonce);
							Self::deposit_event(Event::<T>::TheaPayloadProcessed(
								msg.message.network,
								msg.message.nonce,
							));
							// Save the incoming message for some time
							<IncomingMessages<T>>::insert(
								msg.message.network,
								msg.message.nonce,
								msg.message,
							);
							if let Err(err) = T::NativeCurrency::release(
								&THEA_HOLD_REASON,
								&msg.relayer,
								msg.stake.saturated_into(),
								Precision::BestEffort,
							) {
								// Emit an error event
								Self::deposit_event(Event::<T>::ErrorWhileReleasingLock(
									msg.relayer,
									err,
								));
							}
						}
					},
				}
			}
			<T as pallet::Config>::WeightInfo::on_initialize(active_networks.len() as u32)
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
				Call::submit_signed_outgoing_messages { auth_index, signatures, id } => {
					Self::validate_signed_outgoing_message(auth_index, id, signatures)
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submit incoming message
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::submit_incoming_message(1))]
		#[transactional]
		pub fn submit_incoming_message(
			origin: OriginFor<T>,
			payload: Message,
			stake: Balance,
		) -> DispatchResult {
			let signer = ensure_signed(origin)?;
			let expected_signer = <AllowListTestingRelayers<T>>::get(payload.network)
				.ok_or(Error::<T>::NoRelayersFound)?;
			ensure!(signer == expected_signer, Error::<T>::NotAnAllowlistedRelayer);

			let config = <NetworkConfig<T>>::get(payload.network);

			if stake < config.min_stake {
				return Err(Error::<T>::NotEnoughStake.into());
			}

			let next_nonce = <IncomingNonce<T>>::get(payload.network);
			ensure!(payload.nonce > next_nonce, Error::<T>::NonceError);

			match <IncomingMessagesQueue<T>>::get(payload.network, payload.nonce) {
				None => {
					// Lock balance
					T::NativeCurrency::hold(&THEA_HOLD_REASON, &signer, stake.saturated_into())?;
					// Put it in a queue
					<IncomingMessagesQueue<T>>::insert(
						payload.network,
						payload.nonce,
						IncomingMessage {
							message: payload,
							relayer: signer,
							stake,
							execute_at: frame_system::Pallet::<T>::current_block_number()
								.saturated_into::<u32>()
								.saturating_add(config.fork_period),
						},
					);
				},
				Some(mut existing_payload) => {
					// Update the message only if stake is higher.
					if existing_payload.stake < stake {
						T::NativeCurrency::release(
							&THEA_HOLD_REASON,
							&existing_payload.relayer,
							existing_payload.stake.saturated_into(),
							Precision::BestEffort,
						)?;
						T::NativeCurrency::hold(&THEA_HOLD_REASON, &signer, stake.saturated_into())?;
						existing_payload.message = payload;
						existing_payload.relayer = signer;
						existing_payload.stake = stake;
						existing_payload.execute_at =
							frame_system::Pallet::<T>::current_block_number()
								.saturated_into::<u32>()
								.saturating_add(config.fork_period);
						<IncomingMessagesQueue<T>>::insert(
							existing_payload.message.network,
							existing_payload.message.nonce,
							existing_payload,
						);
					}
				},
			}

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
		pub fn add_thea_network(
			origin: OriginFor<T>,
			network: Network,
			is_evm: bool,
			fork_period: u32,
			min_stake: u128,
			fisherman_stake: u128,
		) -> DispatchResult {
			ensure_root(origin)?;
			<NetworkConfig<T>>::insert(
				network,
				thea_primitives::types::NetworkConfig::new(
					fork_period,
					min_stake,
					fisherman_stake,
					is_evm,
				),
			);
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

		/// Signed outgoing messages
		#[pallet::call_index(6)]
		#[pallet::weight(< T as Config >::WeightInfo::submit_signed_outgoing_messages())]
		pub fn submit_signed_outgoing_messages(
			origin: OriginFor<T>,
			auth_index: u32,
			id: thea_primitives::ValidatorSetId,
			signatures: Vec<(Network, u64, T::Signature)>,
		) -> DispatchResult {
			ensure_none(origin)?;

			for (network, nonce, signature) in signatures {
				let message = match <OutgoingMessages<T>>::get(network, nonce) {
					None => return Err(Error::<T>::MessageNotFound.into()),
					Some(msg) => msg,
				};
				match <SignedOutgoingMessages<T>>::get(network, nonce) {
					None => {
						let signed_msg = SignedMessage::new(message, id, auth_index, signature);
						<SignedOutgoingMessages<T>>::insert(network, nonce, signed_msg);
					},
					Some(mut signed_msg) => {
						signed_msg.add_signature(message, id, auth_index, signature);
						let auth_len = <Authorities<T>>::get(signed_msg.validator_set_id).len();
						if signed_msg.threshold_reached(auth_len) {
							<SignedOutgoingNonce<T>>::insert(network, nonce);
							// Emit an event
							Self::deposit_event(Event::<T>::TheaSignatureFinalized(network, nonce));
						}
						let total_signatures = signed_msg.signatures.len();
						<SignedOutgoingMessages<T>>::insert(network, nonce, signed_msg);
						// Emit an event
						Self::deposit_event(Event::<T>::TheaSignatureUpdated(
							network,
							nonce,
							total_signatures as u16,
						));
					},
				}
			}

			Ok(())
		}

		/// Report misbehaviour as fisherman
		#[pallet::call_index(7)]
		#[pallet::weight(< T as Config >::WeightInfo::report_misbehaviour())]
		#[transactional]
		pub fn report_misbehaviour(
			origin: OriginFor<T>,
			network: Network,
			nonce: u64,
		) -> DispatchResult {
			let fisherman = ensure_signed(origin)?;
			let config = <NetworkConfig<T>>::get(network);
			//  Check if min stake is given
			if T::NativeCurrency::reducible_balance(&fisherman, Preservation::Preserve, Fortitude::Polite)
				< config.fisherman_stake.saturated_into()
			{
				return Err(Error::<T>::NotEnoughStake.into());
			}
			T::NativeCurrency::hold(
				&THEA_HOLD_REASON,
				&fisherman,
				config.fisherman_stake.saturated_into(),
			)?;
			// Message from incoming message queue
			match <IncomingMessagesQueue<T>>::take(network, nonce) {
				None => return Err(Error::<T>::MessageNotFound.into()),
				Some(reported_msg) => {
					// Place it in misbehaviour reports
					let report = MisbehaviourReport {
						reported_msg,
						fisherman: fisherman.clone(),
						stake: config.fisherman_stake,
					};
					<MisbehaviourReports<T>>::insert(network, nonce, report);
					// Emit an event
					Self::deposit_event(Event::<T>::MisbehaviourReported(
						fisherman, network, nonce,
					));
				},
			}
			Ok(())
		}

		/// Handle misbehaviour via governance
		#[pallet::call_index(8)]
		#[pallet::weight(< T as Config >::WeightInfo::handle_misbehaviour())]
		#[transactional]
		pub fn handle_misbehaviour(
			origin: OriginFor<T>,
			network: Network,
			nonce: u64,
			acceptance: bool,
		) -> DispatchResult {
			T::TheaGovernanceOrigin::ensure_origin(origin)?;
			match <MisbehaviourReports<T>>::take(network, nonce) {
				None => {},
				Some(report) => {
					if acceptance {
						// Release lock on relayer
						T::NativeCurrency::release(
							&THEA_HOLD_REASON,
							&report.reported_msg.relayer,
							report.reported_msg.stake.saturated_into(),
							Precision::BestEffort,
						)?;
						// Transfer to fisherman
						T::NativeCurrency::transfer(
							&report.reported_msg.relayer,
							&report.fisherman,
							report.reported_msg.stake.saturated_into(),
							Preservation::Expendable,
						)?;
						// Release fisherman lock
						T::NativeCurrency::release(
							&THEA_HOLD_REASON,
							&report.fisherman,
							report.stake.saturated_into(),
							Precision::BestEffort,
						)?;
					} else {
						// Insert back the message to queue
						<IncomingMessagesQueue<T>>::insert(
							report.reported_msg.message.network,
							report.reported_msg.message.nonce,
							report.reported_msg,
						);
						// burn fisherman stake
						T::NativeCurrency::burn_from(
							&report.fisherman,
							report.stake.saturated_into(),
							Precision::BestEffort,
							Fortitude::Force,
						)?;
					}
				},
			}
			Ok(())
		}

		/// Adds a relayer origin for deposits - will be removed after mainnet testing
		#[pallet::call_index(9)]
		#[pallet::weight(< T as Config >::WeightInfo::add_thea_network())]
		pub fn add_relayer_origin_for_network(
			origin: OriginFor<T>,
			network: Network,
			relayer: T::AccountId,
		) -> DispatchResult {
			ensure_root(origin)?;
			<AllowListTestingRelayers<T>>::insert(network, relayer);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	pub fn active_validators() -> Vec<T::TheaId> {
		let id = Self::validator_set_id();
		<Authorities<T>>::get(id).to_vec()
	}

	fn validate_signed_outgoing_message(
		auth_index: &u32,
		id: &thea_primitives::ValidatorSetId,
		signatures: &Vec<(Network, u64, T::Signature)>,
	) -> TransactionValidity {
		let authorities = <Authorities<T>>::get(id).to_vec();
		let signer: &T::TheaId = match authorities.get(*auth_index as usize) {
			None => return InvalidTransaction::Custom(1).into(),
			Some(signer) => signer,
		};
		for (network, nonce, signature) in signatures {
			let next_outgoing_nonce = <SignedOutgoingNonce<T>>::get(network).saturating_add(1);
			if *nonce != next_outgoing_nonce {
				return InvalidTransaction::Custom(2).into();
			}

			// Reject if it contains already submitted message signatures
			match <SignedOutgoingMessages<T>>::get(network, nonce) {
				None => {},
				Some(signed_msg) => {
					if signed_msg.contains_signature(auth_index) {
						return InvalidTransaction::Custom(4).into();
					}
				},
			}

			let message = match <OutgoingMessages<T>>::get(network, nonce) {
				None => return InvalidTransaction::Custom(3).into(),
				Some(msg) => msg,
			};
			let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());

			if !sp_io::crypto::ecdsa_verify_prehashed(
				&signature.clone().into(),
				&msg_hash,
				&signer.clone().into(),
			) {
				return InvalidTransaction::Custom(6).into();
			}
		}

		ValidTransaction::with_tag_prefix("thea")
			.priority(TransactionPriority::MAX / 3)
			.and_provides((id, auth_index))
			.longevity(10)
			.propagate(true)
			.build()
	}

	/// Generates the next payload based on saved nonce,
	///
	/// NOTE: It will not change the nonce on storage.
	pub fn generate_payload(payload_type: PayloadType, network: Network, data: Vec<u8>) -> Message {
		// Generate the Thea payload to communicate with foreign chains
		let nonce = <OutgoingNonce<T>>::get(network);
		Message {
			block_no: frame_system::Pallet::<T>::current_block_number().saturated_into(),
			nonce: nonce.saturating_add(1),
			data,
			network,
			payload_type,
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
			let mut uncompressed_keys: Vec<[u8; 20]> = vec![];
			for public_key in queued.clone().into_iter() {
				let public_key: sp_core::ecdsa::Public = public_key.into();
				if public_key.0 == [0u8; 33] {
					uncompressed_keys.push([0u8; 20]);
					continue;
				}
				if let Ok(compressed_key) = libsecp256k1::PublicKey::parse_compressed(&public_key.0)
				{
					let uncompressed_key = compressed_key.serialize();
					let uncompressed_key: [u8; 64] =
						if let Ok(uncompressed_key) = uncompressed_key[1..65].try_into() {
							uncompressed_key
						} else {
							log::error!(target: "thea", "Unable to slice last 64 bytes of uncompressed_key for Evm");
							Self::deposit_event(Event::<T>::UnableToSlicePublicKeyHash(
								public_key.into(),
							));
							return;
						};
					let hash: [u8; 32] = sp_io::hashing::keccak_256(&uncompressed_key);
					if let Ok(address) = hash[12..32].try_into() {
						uncompressed_keys.push(address);
					} else {
						log::error!(target: "thea", "Unable to slice last 20 bytes of hash for Evm");
						Self::deposit_event(Event::<T>::UnableToSlicePublicKeyHash(
							public_key.into(),
						));
						return;
					}
				} else {
					log::error!(target: "thea", "Unable to parse compressed key");
					Self::deposit_event(Event::<T>::UnableToParsePublicKey(public_key.into()));
					return;
				}
			}
			for network in &active_networks {
				let network_config = <NetworkConfig<T>>::get(*network);
				let message = match network_config.network_type {
					NetworkType::Evm => {
						if let Some(payload) = ValidatorSet::new(uncompressed_keys.clone(), new_id)
						{
							Self::generate_payload(
								PayloadType::ScheduledRotateValidators,
								*network,
								payload.encode(),
							)
						} else {
							log::error!(target: "thea", "Unable to generate rotate validators payload");
							Self::deposit_event(Event::<T>::UnableToGenerateValidatorSet(*network));
							continue;
						}
					},
					NetworkType::Parachain => {
						if let Some(payload) = ValidatorSet::new(queued.clone(), new_id) {
							Self::generate_payload(
								PayloadType::ScheduledRotateValidators,
								*network,
								payload.encode(),
							)
						} else {
							log::error!(target: "thea", "Unable to generate rotate validators payload");
							Self::deposit_event(Event::<T>::UnableToGenerateValidatorSet(*network));
							continue;
						}
					},
				};
				<OutgoingNonce<T>>::insert(message.network, message.nonce);
				<OutgoingMessages<T>>::insert(message.network, message.nonce, message);
			}
			<NextAuthorities<T>>::put(queued);
		}
		if incoming != outgoing {
			// This will happen when new era starts, or end of the last epoch
			<Authorities<T>>::insert(new_id, incoming);
			<ValidatorSetId<T>>::put(new_id);
			for network in active_networks {
				let message =
					Self::generate_payload(PayloadType::ValidatorsRotated, network, Vec::new()); //Empty data means acitvate the next set_id
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
		let payload = Self::generate_payload(PayloadType::L1Deposit, network, data);
		// Update nonce
		<OutgoingNonce<T>>::insert(network, payload.nonce);
		<OutgoingMessages<T>>::insert(network, payload.nonce, payload);
		Ok(())
	}
}
