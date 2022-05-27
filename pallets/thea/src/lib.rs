// This file is part of Polkadex.

// Copyright (C) 2020-2021 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// NOTE: This pallet is modified for thea pallet from Paritytech's beefy.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Encode;
use frame_support::traits::{OneSessionHandler, ValidatorSetWithIdentification, ValidatorSet};
use frame_support::storage::{IterableStorageMap, bounded_btree_set::BoundedBTreeSet};
use scale_info::TypeInfo;
use sp_runtime::{
	generic::DigestItem,
	traits::{IsMember, Saturating, UniqueSaturatedInto, Convert, StaticLookup, CheckedConversion},
	AccountId32, Perbill, RuntimeAppPublic, RuntimeDebug,
};
use sp_staking::{
	offence::{Kind, Offence, ReportOffence},
	SessionIndex,
};
use sp_core::crypto::UncheckedInto;
use sp_std::prelude::*;

pub use pallet::*;
use thea_primitives::{
	keygen::{KeygenRound, OffenseReport, OfflineStageRound, SigningSessionPayload, TheaPayload, SubProtocol},
	payload::{Network, SignedTheaPayload, UnsignedTheaPayload},
	AuthorityIndex, ConsensusLog, PartyIndex, THEA_ENGINE_ID, OffenceReportBTreeSetLimit
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// A type for representing the validator id in a session.
pub type ValidatorId<T> = <<T as Config>::ValidatorSet as ValidatorSet<
	<T as frame_system::Config>::AccountId,
>>::ValidatorId;

/// A tuple of (ValidatorId, Identification) where `Identification` is the full identification of
/// `ValidatorId`.
pub type IdentificationTuple<T> = (
	ValidatorId<T>,
	<<T as Config>::ValidatorSet as ValidatorSetWithIdentification<
		<T as frame_system::Config>::AccountId,
	>>::Identification,
);

// TODO: Should this be arbitrary value?
const UNSIGNED_TXS_PRIORITY: u64 = 100;

mod submit;
pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::{InvalidTransaction, *};
	use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};

	use sp_core::ecdsa::Public;
	use sp_std::{prelude::*, result};

	use thea_primitives::{
		inherents::{InherentError, TheaPublicKeyInherentDataType, INHERENT_IDENTIFIER},
		keygen::SigningSessionPayload,
		traits::HandleSignedPayloadTrait,
	};

	use super::*;

	pub trait TheaWeightInfo {
		fn submit_keygen_message(b: u32) -> Weight;
		fn clean_keygen_messages(b: u32) -> Weight;
		fn submit_offline_message(b: u32) -> Weight;
		fn submit_signing_message(_b: u32) -> Weight;
		fn submit_signed_payload(_b: u32) -> Weight;
		fn submit_ecdsa_public_key(_b: u32) -> Weight;
		fn register_deposit_address() -> Weight;
	}

	/// Enum for InvalidTransaction::Custom use
	pub enum InvalidTransactions {
		TooManyMessages = 10,
	}

	#[pallet::config]
	pub trait Config: frame_system::Config + CreateSignedTransaction<Call<Self>> {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Authority identifier type
		type TheaId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ Ord
			+ MaxEncodedLen;
		/// Ethereum Handler
		type EthereumHandler: HandleSignedPayloadTrait;
		/// The maximum length for an unsigned payload
		#[pallet::constant]
		type PayloadLimit: Get<u32>;
		/// Weights for the pallet
		type TheaWeightInfo: TheaWeightInfo;
		/// A type for retrieving the validators supposed to be participating in a session
		type ValidatorSet: ValidatorSetWithIdentification<Self::AccountId>;
		/// A type that gives us the ability to submit offence reports.
		type ReportMisbehaviour: ReportOffence<
			Self::AccountId,
			IdentificationTuple<Self>,
			UnresponsivenessOffence<IdentificationTuple<Self>>,
		>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::error]
	pub enum Error<T> {
		SignerNotFound,
		OffenderNotFound,
		BoundOverflow,
	}

	#[pallet::inherent]
	impl<T: Config> ProvideInherent for Pallet<T> {
		type Call = Call<T>;
		type Error = InherentError;
		const INHERENT_IDENTIFIER: InherentIdentifier = INHERENT_IDENTIFIER;

		fn create_inherent(data: &InherentData) -> Option<Self::Call> {
			if let Ok(inherent_data) =
				data.get_data::<TheaPublicKeyInherentDataType>(&INHERENT_IDENTIFIER)
			{
				return match inherent_data {
					None => None,
					Some(inherent_data)
						if inherent_data.public_key.is_some() &&
							inherent_data.public_key.clone().unwrap() != Public([0u8;33]) =>
					{
						// We don't need to set the inherent data every block, it is only needed
						// once.
						let pubk = <PublicKeys<T>>::get(inherent_data.set_id);
						if pubk == None {
							Some(Call::submit_ecdsa_public_key {
								set_id: inherent_data.set_id,
								public_key: inherent_data.public_key.unwrap(),
							})
						} else {
							None
						}
					}
					_ => None,
				}
			}
			None
		}

		fn check_inherent(
			call: &Self::Call,
			data: &InherentData,
		) -> result::Result<(), Self::Error> {
			let (set_id, publickey) = match call {
				Call::submit_ecdsa_public_key { set_id, public_key } => (set_id, public_key),
				_ => return Err(InherentError::WrongInherentCall),
			};

			let expected_data = data
				.get_data::<TheaPublicKeyInherentDataType>(&INHERENT_IDENTIFIER)
				.expect("Thea inherent data not correctly encoded")
				.expect("Thea inherent data must be provided");

			if &expected_data.set_id == set_id &&
				expected_data.public_key.is_some() &&
				&expected_data.public_key.unwrap() != publickey
			{
				return Err(InherentError::InvalidPublicKey(TheaPublicKeyInherentDataType {
					public_key: Some(publickey.clone()),
					set_id: *set_id,
				}))
			}

			Ok(())
		}

		fn is_inherent(call: &Self::Call) -> bool {
			matches!(call, Call::submit_ecdsa_public_key { .. })
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let valid_tx = |provide, rng: u64| {
				ValidTransaction::with_tag_prefix("thea-proc")
					.priority(rng)
					.and_provides([&(&provide, rng.to_be())])
					.longevity(3)
					.propagate(true)
					.build()
			};

			let valid_offence_tx = | rng: u64| {
				ValidTransaction::with_tag_prefix("thea-proc")
					.priority(rng)
					.and_provides([&(rng.to_be())])
					.longevity(3)
					.propagate(true)
					.build()
			};

			let valid_signed_payload = |provide, rng: u64| {
				ValidTransaction::with_tag_prefix("thea-proc")
					.priority(rng)
					.and_provides([&(&provide, rng.to_be())])
					.longevity(3)
					.propagate(true)
					.build()
			};

			let valid_inherent = || {
				ValidTransaction::with_tag_prefix("thea-proc")
					.priority(UNSIGNED_TXS_PRIORITY + 1)
					.and_provides([&(b"ecdsa_pubk_inherent")])
					.longevity(1)
					.propagate(false)
					.build()
			};

			let valid_auth_keygen = |authorities: Vec<T::TheaId>,
			                         payload: &TheaPayload<
				T::TheaId,
				KeygenRound,
				thea_primitives::MsgLimit,
				thea_primitives::MsgVecLimit,
			>,
			                         signature: &<T::TheaId as RuntimeAppPublic>::Signature,
			                         rng: u64|
			 -> TransactionValidity {
				if payload.messages.len() > (Self::message_limit() as usize) {
					return InvalidTransaction::Custom(InvalidTransactions::TooManyMessages as u8)
						.into()
				}
				let authority_id = match authorities.get(payload.auth_idx as usize) {
					Some(val) => val.clone(),
					None => return InvalidTransaction::BadProof.into(),
				};
				let message_hash = sp_io::hashing::keccak_256(&payload.encode());

				// verify if a valid authority signed this transaction
				if !authority_id.verify(&message_hash, signature) {
					return InvalidTransaction::BadProof.into()
				}

				// Make sure the provides tag is different.
				valid_tx((authority_id, payload.auth_idx), rng)
			};

			let valid_auth_clean_keygen =
				|authorities: Vec<T::TheaId>,
				 auth_idx: AuthorityIndex,
				 signature: &<T::TheaId as RuntimeAppPublic>::Signature,
				 rng: u64|
				 -> TransactionValidity {
					let authority_id = match authorities.get(auth_idx as usize) {
						Some(val) => val.clone(),
						None => return InvalidTransaction::BadProof.into(),
					};
					let message_hash = sp_io::hashing::keccak_256(&auth_idx.encode());
					// verify if a valid authority signed this transaction
					if !authority_id.verify(&message_hash, signature) {
						return InvalidTransaction::BadProof.into()
					}
					valid_tx((authority_id, auth_idx), rng)
				};

			let valid_auth_offline = |authorities: Vec<T::TheaId>,
			                          payload: &TheaPayload<
				T::TheaId,
				OfflineStageRound,
				thea_primitives::MsgLimit,
				thea_primitives::MsgVecLimit,
			>,
			                          signature: &<T::TheaId as RuntimeAppPublic>::Signature,
			                          rng: u64|
			 -> TransactionValidity {
				if payload.messages.len() > (Self::message_limit() as usize) {
					return InvalidTransaction::Custom(InvalidTransactions::TooManyMessages as u8)
						.into()
				}
				let authority_id = match authorities.get(payload.auth_idx as usize) {
					Some(val) => val.clone(),
					None => return InvalidTransaction::BadProof.into(),
				};
				let message_hash = sp_io::hashing::keccak_256(&payload.encode());

				// verify if a valid authority signed this transaction
				if !authority_id.verify(&message_hash, signature) {
					return InvalidTransaction::BadProof.into()
				}

				// Make sure the provides tag is different.
				valid_tx((authority_id, payload.auth_idx), rng)
			};

			let valid_auth_signing = |authorities: Vec<T::TheaId>,
			                          payload: &SigningSessionPayload<
				T::TheaId,
				thea_primitives::PartialSignatureLimit,
				thea_primitives::PartialSignatureVecLimit,
			>,
			                          signature: &<T::TheaId as RuntimeAppPublic>::Signature,
			                          rng: u64|
			 -> TransactionValidity {
				if payload.partial_signatures.len() > (Self::message_limit() as usize) {
					return InvalidTransaction::Custom(InvalidTransactions::TooManyMessages as u8)
						.into()
				}
				let authority_id = match authorities.get(payload.auth_idx as usize) {
					Some(val) => val.clone(),
					None => return InvalidTransaction::BadProof.into(),
				};
				let message_hash = sp_io::hashing::keccak_256(&payload.encode());

				// verify if a valid authority signed this transaction
				if !authority_id.verify(&message_hash, signature) {
					return InvalidTransaction::BadProof.into()
				}

				// Make sure the provides tag is different.
				valid_tx((authority_id, payload.auth_idx), rng)
			};

			// Loop through all the payloads and verify the signature
			let verify_signed_payload = |payload: &SignedTheaPayload,
			                             rng: u64|
			 -> TransactionValidity {
				let _vid = Self::validator_set_id();
				let pubk = <PublicKeys<T>>::get(Self::validator_set_id());
				if pubk != None {
					// TODO: Make this expensive computation loop parallel, check https://github.com/paritytech/substrate/tree/master/frame/example-parallel
					let is_valid = match thea_primitives::runtime::crypto::verify_ecdsa_prehashed(
						&payload.signature,
						&pubk.unwrap(),
						&payload.payload.payload,
					) {
						true => true,
						// fallback to non pre-hashed verification
						false => thea_primitives::runtime::crypto::verify_ecdsa(
							&payload.signature,
							&pubk.unwrap(),
							&payload.payload.payload,
						),
					};
					if !is_valid {
						// Note fail the entire transaction even if one signature fails.
						return InvalidTransaction::BadProof.into()
					}
					valid_signed_payload(payload.payload.payload, rng)
				} else {
					InvalidTransaction::Call.into()
				}
			};

			match call {
				/* Call::submit_payload { network: _, payload, rng } =>
				valid_signed_payload(*payload, *rng), */
				Call::submit_keygen_message { payload, signature, rng } => {
					let current_set_id = Self::validator_set_id();

					if payload.set_id == current_set_id {
						valid_auth_keygen(Self::authorities(), payload, signature, *rng)
					} else if payload.set_id == current_set_id + 1 {
						valid_auth_keygen(Self::next_authorities(), payload, signature, *rng)
					} else {
						InvalidTransaction::Call.into()
					}
				},
				Call::submit_offline_message { payload, signature, rng, payload_array: _ } => {
					let current_set_id = Self::validator_set_id();

					if payload.set_id == current_set_id {
						valid_auth_offline(Self::authorities(), payload, signature, *rng)
					} else if payload.set_id == current_set_id + 1 {
						valid_auth_offline(Self::next_authorities(), payload, signature, *rng)
					} else {
						InvalidTransaction::Call.into()
					}
				},
				Call::submit_signing_message { at: _, payload, signature, rng } => {
					let current_set_id = Self::validator_set_id();

					if payload.set_id == current_set_id {
						valid_auth_signing(Self::authorities(), payload, signature, *rng)
					} else {
						InvalidTransaction::Call.into()
					}
				},

				Call::submit_signed_payload { payload, rng } =>
					verify_signed_payload(payload, *rng),

				// TODO: Is this okay? Anyone can now sent this extrinsic,
				// 		There should be some kind of check in validate unsigned that will prevent spam
				Call::submit_ecdsa_public_key { set_id: _, public_key: _ } => valid_inherent(),
				Call::clean_keygen_messages { auth_idx, signature, rng } =>
					valid_auth_clean_keygen(Self::authorities(), *auth_idx, signature, *rng),
				Call::register_offense{ signature, offence} => {
					// TODO: Need to add some validation using signature
					valid_offence_tx(1)
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Submits a keygen protocol broadcast messages to runtime storage
		#[pallet::weight((<T as pallet::Config>::TheaWeightInfo::submit_keygen_message(1), DispatchClass::Operational))]
		pub fn submit_keygen_message(
			origin: OriginFor<T>,
			payload: TheaPayload<
				T::TheaId,
				KeygenRound,
				thea_primitives::MsgLimit,
				thea_primitives::MsgVecLimit,
			>,
			_signature: <T::TheaId as RuntimeAppPublic>::Signature,
			_rng: u64,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			ensure!(payload.signer.is_some(), Error::<T>::SignerNotFound);
			if let Some(signer) = &payload.signer {
				<KeygenMessages<T>>::insert(payload.auth_idx, payload.round, &payload);
				Self::deposit_event(Event::KeygenMessages(signer.clone(), payload));
			}

			Ok(().into())
		}

		///Clean Keygen and Offline Messages
		#[pallet::weight((<T as pallet::Config>::TheaWeightInfo::clean_keygen_messages(1), DispatchClass::Operational))]
		pub fn clean_keygen_messages(
			origin: OriginFor<T>,
			auth_idx: AuthorityIndex,
			_signature: <T::TheaId as RuntimeAppPublic>::Signature,
			_rng: u64,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			<KeygenMessages<T>>::remove_prefix(auth_idx, None);
			<OfflineMessages<T>>::remove_prefix(auth_idx, None);
			Self::deposit_event(Event::MessagesCleaned(auth_idx));

			Ok(().into())
		}

		/// Submits a offline protocol broadcast messages to runtime storage
		#[pallet::weight((<T as pallet::Config>::TheaWeightInfo::submit_offline_message(1), DispatchClass::Operational))]
		pub fn submit_offline_message(
			origin: OriginFor<T>,
			payload: TheaPayload<
				T::TheaId,
				OfflineStageRound,
				thea_primitives::MsgLimit,
				thea_primitives::MsgVecLimit,
			>,
			payload_array: [u8; 32],
			_signature: <T::TheaId as RuntimeAppPublic>::Signature,
			_rng: u64,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			ensure!(payload.signer.is_some(), Error::<T>::SignerNotFound);
			if let Some(signer) = &payload.signer {
				<OfflineMessages<T>>::insert(
					payload.auth_idx,
					(payload.round, payload_array),
					&payload,
				);
				Self::deposit_event(Event::OfflineMessages(signer.clone(), payload));
			}

			Ok(().into())
		}

		/// Submits a signing protocol broadcast messages to runtime storage
		#[pallet::weight((<T as pallet::Config>::TheaWeightInfo::submit_signing_message(1), DispatchClass::Operational))]
		pub fn submit_signing_message(
			origin: OriginFor<T>,
			at: T::BlockNumber,
			payload: SigningSessionPayload<
				T::TheaId,
				thea_primitives::PartialSignatureLimit,
				thea_primitives::PartialSignatureVecLimit,
			>,
			_signature: <T::TheaId as RuntimeAppPublic>::Signature,
			_rng: u64,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			ensure!(payload.signer.is_some(), Error::<T>::SignerNotFound);
			if let Some(signer) = &payload.signer {
				<SigningMessages<T>>::insert(at, payload.auth_idx, &payload);
				Self::deposit_event(Event::SigningMessages(signer.clone(), payload));
			}

			Ok(().into())
		}

		/// Signed payloads are handled by this function

		#[pallet::weight((<T as pallet::Config>::TheaWeightInfo::submit_signed_payload(1), DispatchClass::Operational))]
		pub fn submit_signed_payload(
			origin: OriginFor<T>,
			payload: SignedTheaPayload,
			_rng: u64,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			match payload.payload.network {
				Network::ETHEREUM => T::EthereumHandler::handle_signed_payload(payload.clone()),
				Network::NONE => {},
			}
			Self::deposit_event(Event::SignedPayloadSubmitted(payload.payload.payload));
			Ok(().into())
		}

		// NOTE: This extrinsic is meant for Testingg only
		/* #[pallet::weight(10_000)]
		pub fn submit_payload(
			origin: OriginFor<T>,
			network: Network,
			payload: [u8; 32],
			_rng: u64,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let now: T::BlockNumber = frame_system::Pallet::<T>::current_block_number();
			// TODO: Make sure the vector is bounded
			let mut unsigned_payloads = <UnsignedPayloads<T>>::get(now);
			unsigned_payloads.push(UnsignedTheaPayload {
				network,
				payload,
				submission_blk: now.unique_saturated_into(),
			});
			<UnsignedPayloads<T>>::insert(now, unsigned_payloads);
			Self::deposit_event(Event::PayloadSubmitted(payload));
			Ok(().into())
		} */

		/// Submits the ecdsa public key to runtime
		#[pallet::weight((<T as pallet::Config>::TheaWeightInfo::submit_ecdsa_public_key(1), DispatchClass::Operational))]
		pub fn submit_ecdsa_public_key(
			origin: OriginFor<T>,
			set_id: thea_primitives::ValidatorSetId,
			public_key: Public,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			<PublicKeys<T>>::insert(set_id, &public_key);
			Self::deposit_event(Event::ECDSAKeySet(set_id, public_key));

			Ok(().into())
		}

		/// Register a new deposit address
		#[pallet::weight(<T as pallet::Config>::TheaWeightInfo::register_deposit_address())]
		pub fn register_deposit_address(origin: OriginFor<T>) -> DispatchResultWithPostInfo {
			let signer = ensure_signed(origin)?;
			<RegisteredDepositAddress<T>>::insert(&signer, &signer);
			Self::deposit_event(Event::NewDepositAddressRegistered(signer));
			Ok(().into())
		}

		/// Register a new deposit address
		#[pallet::weight(10_000)]
		pub fn register_offense(
			_origin: OriginFor<T>,
			_signature: <T::TheaId as RuntimeAppPublic>::Signature,
			offence: OffenseReport<T::AccountId>,
		) -> DispatchResultWithPostInfo {
			let offender = offence.offender.clone(); 
			let round = offence.protocol.clone();

			if <OffenceReport<T>>::contains_key((&offender, &round)) {
			// Update reporters storage
				let mut reporters = <Reporter<T>>::get((&offender, &round)).unwrap();
				if let Ok(_) = reporters.try_insert(offence.author.clone()){
					<Reporter<T>>::insert((&offender, &round), reporters);
				}
				else{
					// Index is out of bounds or author already exists 
				}
			} else {
				let mut reporters:BoundedBTreeSet<T::AccountId, OffenceReportBTreeSetLimit> = BoundedBTreeSet::new();
				reporters.try_insert(offence.author.clone()); 
				<OffenceReport<T>>::insert((&offender, &round), &offence);
				<Reporter<T>>::insert((&offender, &round),reporters);
			}
			Ok(().into())
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn keygen_messages)]
	/// sender, KeygenRound => Messages
	pub(super) type KeygenMessages<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PartyIndex,
		Blake2_128Concat,
		KeygenRound,
		TheaPayload<
			T::TheaId,
			KeygenRound,
			thea_primitives::MsgLimit,
			thea_primitives::MsgVecLimit,
		>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn offline_messages)]
	/// sender, OfflineStageRound => Messages
	pub(super) type OfflineMessages<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PartyIndex,
		Blake2_128Concat,
		(OfflineStageRound, [u8; 32]),
		TheaPayload<
			T::TheaId,
			OfflineStageRound,
			thea_primitives::MsgLimit,
			thea_primitives::MsgVecLimit,
		>,
		ValueQuery,
	>;
	// Ext => (party_index and Sig)
	// Keygen Messages => Party_idx
	// Offline

	#[pallet::storage]
	#[pallet::getter(fn signing_messages)]
	/// sender, OfflineStageRound => Messages
	pub(super) type SigningMessages<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		Blake2_128Concat,
		PartyIndex,
		SigningSessionPayload<
			T::TheaId,
			thea_primitives::PartialSignatureLimit,
			thea_primitives::PartialSignatureVecLimit,
		>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn unsigned_payloads)]
	/// BlockNumber, Network => Vec<Data>
	///
	/// TODO: The vector needs to be bounded
	pub(super) type UnsignedPayloads<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<UnsignedTheaPayload, T::PayloadLimit>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn signed_payloads)]
	/// BlockNumber, Network => Vec<Data>
	///
	/// TODO: The vector needs to be bounded
	pub(super) type SignedPayloads<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<SignedTheaPayload, T::PayloadLimit>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn offence_report)]
	/// (AccountId, KeygenRound) => Report
	pub(super) type OffenceReport<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(T::AccountId, SubProtocol),
		OffenseReport<T::AccountId>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn reporters)]
	/// (AccountId, KeygenRound) => Report
	pub(super) type Reporter<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		(T::AccountId, SubProtocol),
		BoundedBTreeSet<T::AccountId, OffenceReportBTreeSetLimit>,
		OptionQuery,
	>;

	/// The current authorities set
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> =
		StorageValue<_, sp_std::vec::Vec<T::TheaId>, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, thea_primitives::ValidatorSetId, ValueQuery>;

	/// Authorities set scheduled to be used with the next session
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub(super) type NextAuthorities<T: Config> =
		StorageValue<_, sp_std::vec::Vec<T::TheaId>, ValueQuery>;

	/// ECDSA Public keys of each ValidatorSetId
	#[pallet::storage]
	#[pallet::getter(fn public_keys)]
	pub(super) type PublicKeys<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		thea_primitives::ValidatorSetId,
		sp_core::ecdsa::Public,
		OptionQuery,
	>;

	/// Users who registered their deposit addresses, which are available on all networks
	#[pallet::storage]
	#[pallet::getter(fn registered_deposit_addresses)]
	pub(super) type RegisteredDepositAddress<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn validator_set_changed)]
	pub(super) type IsValidatorSetChanged<T: Config> = StorageValue<_, bool, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn message_limit)]
	pub(super) type MessageLimit<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub authorities: Vec<T::TheaId>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { authorities: Vec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_authorities(&self.authorities);
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New Deposit Address registered
		NewDepositAddressRegistered(T::AccountId),
		/// Dummy event, just here so there's a generic type that's used.
		TheaAuthoritySetChange(Vec<T::TheaId>),
		/// THEA keygen Protocol events
		KeygenMessages(
			T::TheaId,
			TheaPayload<
				T::TheaId,
				KeygenRound,
				thea_primitives::MsgLimit,
				thea_primitives::MsgVecLimit,
			>,
		),
		/// THEA Offline Stage Protocol events
		OfflineMessages(
			T::TheaId,
			TheaPayload<
				T::TheaId,
				OfflineStageRound,
				thea_primitives::MsgLimit,
				thea_primitives::MsgVecLimit,
			>,
		),
		/// THEA Signing Round Protocol events
		SigningMessages(
			T::TheaId,
			SigningSessionPayload<
				T::TheaId,
				thea_primitives::PartialSignatureLimit,
				thea_primitives::PartialSignatureVecLimit,
			>,
		),
		/// Thea ECDSA Public key
		ECDSAKeySet(thea_primitives::ValidatorSetId, Public),
		/// Set a given number of signed payloads for given network
		SignedPayload(Vec<SignedTheaPayload>),
		/// All messages for given authority id are removed from storage
		MessagesCleaned(u16),
		/// Indicates new unsigned Thea payload submitting success
		PayloadSubmitted([u8; 32]),
		/// Indicates number of successfully submitted signed payloads
		SignedPayloadSubmitted([u8; 32]),
	}
}

impl<T: Config> Pallet<T> {
	pub fn submit_payload_for_signing(
		now: T::BlockNumber,
		network: Network,
		payload: [u8; 32],
	) -> Result<(), &'static str> {
		return match <UnsignedPayloads<T>>::get(now).try_mutate(
			|vector: &mut Vec<UnsignedTheaPayload>| {
				vector.push(UnsignedTheaPayload {
					network,
					payload,
					submission_blk: now.unique_saturated_into(),
				})
			},
		) {
			Some(unsigned_payloads) => {
				<UnsignedPayloads<T>>::insert(now, unsigned_payloads);
				Ok(())
			},
			None => Err("Out of Bounds"),
		}
	}

	/// Returns all the unsigned payloads for all networks
	pub fn unsigned_payloads_api(at: T::BlockNumber) -> Vec<UnsignedTheaPayload> {
		<UnsignedPayloads<T>>::get(at).to_vec()
	}

	/// Returns all signed payloads for all networks
	pub fn signed_payload_api(at: T::BlockNumber) -> Vec<SignedTheaPayload> {
		<SignedPayloads<T>>::get(at).to_vec()
	}

	/// Return the signing stage messages for given round and party index
	pub fn signing_messages_api(
		at: T::BlockNumber,
	) -> Vec<
		SigningSessionPayload<
			T::TheaId,
			thea_primitives::PartialSignatureLimit,
			thea_primitives::PartialSignatureVecLimit,
		>,
	> {
		<SigningMessages<T>>::iter_prefix_values(at).collect()
	}
	/// Return the offline stage messages for given round and party index
	pub fn offline_messages_api(
		party_idx: PartyIndex,
		round: OfflineStageRound,
		payload: &[u8; 32],
	) -> TheaPayload<
		T::TheaId,
		OfflineStageRound,
		thea_primitives::MsgLimit,
		thea_primitives::MsgVecLimit,
	> {
		<OfflineMessages<T>>::get(party_idx, (round, *payload))
	}
	/// Return the keygen messages for given round and party index
	pub fn keygen_messages_api(
		party_idx: PartyIndex,
		round: KeygenRound,
	) -> TheaPayload<T::TheaId, KeygenRound, thea_primitives::MsgLimit, thea_primitives::MsgVecLimit>
	{
		<KeygenMessages<T>>::get(party_idx, round)
	}

	/// Return the current active THEA validator set.
	pub fn validator_set() -> thea_primitives::ValidatorSet<T::TheaId> {
		let current_set_id = Self::validator_set_id();
		let public_key = Self::public_keys(current_set_id);
		thea_primitives::ValidatorSet::<T::TheaId> {
			validators: Self::authorities(),
			id: current_set_id,
			public_key,
		}
	}

	/// Submit an offence report to runtime
	pub fn submit_offence_report(report: OffenseReport<T::AccountId>){
		// TODO! Need to write logic to perform this 
		let offender = report.offender.clone(); 
		let round = report.protocol.clone();

		if <OffenceReport<T>>::contains_key((&offender, &round)) {
			// Update reporters storage
			let mut reporters = <Reporter<T>>::get((&offender, &round)).unwrap();
			if let Ok(_) = reporters.try_insert(report.author.clone()){
				<Reporter<T>>::insert((&offender, &round), reporters);
			}
			else{
				// Index is out of bounds or author already exists 
			}
		} else {
			let mut reporters:BoundedBTreeSet<T::AccountId, OffenceReportBTreeSetLimit> = BoundedBTreeSet::new();
			reporters.try_insert(report.author.clone()); 
			<OffenceReport<T>>::insert((&offender, &report.protocol), &report);
			<Reporter<T>>::insert((&offender, &round),reporters);
		}
	}

	/// Verifies the report submitted by validators
	pub fn verify_report(report: OffenseReport<T::AccountId>) -> bool {
		// TODO! We need to write logic to verify reports submitted by Validators
		let offender = report.offender.clone(); 
		let threshold = (2/3)*<Authorities<T>>::get().len();
		let round = report.protocol; 
		if <Reporter<T>>::contains_key((&offender, &round)){
			let reporter = <Reporter<T>>::get((&offender, &round)).unwrap();
			if reporter.len() >= threshold {
				return true;
			}

		} 
		false
	}

	/// Return the nexts active THEA validator set.
	pub fn next_validator_set() -> thea_primitives::ValidatorSet<T::TheaId> {
		let id = Self::validator_set_id() + 1;
		let public_key = Self::public_keys(id);
		thea_primitives::ValidatorSet::<T::TheaId> { validators: Self::next_authorities(), id, public_key }
	}

	/// Return true if last Epoch and Validator Set Changed otherwise False
	pub fn is_validator_set_changed() -> bool {
		Self::validator_set_changed()
	}

	#[allow(dead_code)]
	fn change_authorities(new: Vec<T::TheaId>, queued: Vec<T::TheaId>) {
		// As in GRANDPA, we trigger a validator set change only if the the validator
		// set has actually changed.
		if new != Self::authorities() && new.len() > 3 {
			<Authorities<T>>::put(&new);

			let next_id = Self::validator_set_id() + 1u64;
			<ValidatorSetId<T>>::put(next_id);

			let log: DigestItem = DigestItem::Consensus(
				THEA_ENGINE_ID,
				ConsensusLog::AuthoritiesChange(thea_primitives::ValidatorSet {
					validators: new,
					id: next_id,
					public_key: None,
				})
				.encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(log);
		}

		<NextAuthorities<T>>::put(&queued);
		Self::deposit_event(Event::TheaAuthoritySetChange(queued));
	}

	fn initialize_authorities(authorities: &[T::TheaId]) {
		if authorities.is_empty() {
			return
		}

		// assert!(
		// 	authorities.len() >= 3,
		// 	"Minimum number of thea authorities are 3, provided: {:?}",
		// 	authorities.len()
		// );
		assert!(<Authorities<T>>::get().is_empty(), "Authorities are already initialized!");

		<Authorities<T>>::put(authorities);
		<ValidatorSetId<T>>::put(0);
		// Like `pallet_session`, initialize the next validator set as well.
		<NextAuthorities<T>>::put(authorities);
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::TheaId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T>  {
	type Key = T::TheaId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::TheaId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		let limit: u64 = authorities.len().unique_saturated_into();
		<MessageLimit<T>>::put(limit);
		Self::initialize_authorities(&authorities);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::TheaId)>,
	{
		let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();

		if next_queued_authorities != next_authorities {
			<IsValidatorSetChanged<T>>::put(true);
			<NextAuthorities<T>>::put(next_queued_authorities);
		} else {
			<IsValidatorSetChanged<T>>::put(false);
		}

		// if changed => CurrentVladotr =>
		if changed {
			let limit: u64 = next_authorities.len().unique_saturated_into();
			<MessageLimit<T>>::put(limit);
			<Authorities<T>>::put(next_authorities);
		}
	}
	fn on_before_session_ending(){
		let session_index =  T::ValidatorSet::session_index();
		let current_validators = T::ValidatorSet::validators();
		// This entire block of code can be handled better
		// Iterate over all the offenses
		let offenders_id = <OffenceReport<T>>::iter_values()
			.filter(|report|{
				Self::verify_report(report.clone())
			})
			.map(|report| report.offender.clone())
			.map(|offender| <<T as Config>::ValidatorSet as ValidatorSet<<T as frame_system::Config>::AccountId,>>::ValidatorIdOf::convert(offender.clone()).unwrap() )
			.collect::<Vec<ValidatorId<T>>>();

		// Iterate over all the validators
		let offenders = current_validators
				.into_iter()
				.enumerate()
				.filter(|(index, id)| offenders_id.contains(id))
				.filter_map(|(_, id)|{
					<T::ValidatorSet as ValidatorSetWithIdentification<T::AccountId>>::IdentificationOf::convert(
						id.clone()
					).map(|full_id|(id, full_id))
				})
				.collect::<Vec<IdentificationTuple<T>>>();
		

		if !offenders.is_empty(){
			let validator_set_count = Self::authorities().len() as u32;
			let offence = UnresponsivenessOffence { session_index, validator_set_count, offenders: offenders.clone() };
			if let Err(e) = T::ReportMisbehaviour::report_offence(vec![], offence) {
				sp_runtime::print(e);
			}	
		}
	}

	fn on_disabled(i: u32) {
		let log: DigestItem = DigestItem::Consensus(
			THEA_ENGINE_ID,
			ConsensusLog::<T::TheaId>::OnDisabled(i as AuthorityIndex).encode(),
		);

		<frame_system::Pallet<T>>::deposit_log(log);
	}
}

impl<T: Config> IsMember<T::TheaId> for Pallet<T> {
	fn is_member(authority_id: &T::TheaId) -> bool {
		Self::authorities().iter().any(|id| id == authority_id)
	}
}
// An offence that is filed if a validator didn't send a keygen message.
#[derive(RuntimeDebug, TypeInfo)]
#[cfg_attr(feature = "std", derive(Clone, PartialEq, Eq))]
pub struct UnresponsivenessOffence<Offender> {
	/// The current session index in which we report the unresponsive validators.
	///
	/// It acts as a time measure for unresponsiveness reports and effectively will always point
	/// at the end of the session.
	pub session_index: SessionIndex,
	/// The size of the validator set in current session/era.
	pub validator_set_count: u32,
	/// Authorities that were unresponsive during the current era.
	pub offenders: Vec<Offender>,
}

impl<Offender: Clone> Offence<Offender> for UnresponsivenessOffence<Offender> {
	const ID: Kind = *b"im-online:offlin";
	type TimeSlot = SessionIndex;

	fn offenders(&self) -> Vec<Offender> {
		self.offenders.clone()
	}

	fn session_index(&self) -> SessionIndex {
		self.session_index
	}

	fn validator_set_count(&self) -> u32 {
		self.validator_set_count
	}

	fn time_slot(&self) -> Self::TimeSlot {
		self.session_index
	}

	fn slash_fraction(offenders: u32, validator_set_count: u32) -> Perbill {
		// the formula is min((3 * (k - (n / 10 + 1))) / n, 1) * 0.07
		// basically, 10% can be offline with no slash, but after that, it linearly climbs up to 7%
		// when 13/30 are offline (around 5% when 1/3 are offline).
		if let Some(threshold) = offenders.checked_sub(validator_set_count / 10 + 1) {
			let x = Perbill::from_rational(3 * threshold, validator_set_count);
			x.saturating_mul(Perbill::from_percent(7))
		} else {
			Perbill::default()
		}
	}
}
