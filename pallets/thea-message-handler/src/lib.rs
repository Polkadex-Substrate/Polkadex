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

//! # Thea Message Handler Pallet.
//!
//! Pallet which processes incoming messages.
//!
//! Used only by "Parachain".

use frame_support::{pallet_prelude::*, traits::Get, BoundedVec, Parameter};
use frame_system::pallet_prelude::*;
pub use pallet::*;
use parity_scale_codec::Encode;
use sp_runtime::{
	traits::{BlockNumberProvider, Member},
	transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	Percent, RuntimeAppPublic, SaturatedConversion,
};
use sp_std::prelude::*;
use thea_primitives::{types::Message, Network, ValidatorSet};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
pub mod test;

pub trait WeightInfo {
	fn insert_authorities(_b: u32) -> Weight;
	fn incoming_message() -> Weight;
	fn update_incoming_nonce(_b: u32) -> Weight;
	fn update_outgoing_nonce(_b: u32) -> Weight;
}

pub mod weights;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::transactional;
	use sp_std::vec;
	use thea_primitives::{types::Message, TheaIncomingExecutor};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Authority identifier type
		type TheaId: Member + Parameter + RuntimeAppPublic + MaybeSerializeDeserialize;

		/// Authority Signature
		type Signature: IsType<<Self::TheaId as RuntimeAppPublic>::Signature> + Member + Parameter;

		/// The maximum number of authorities that can be added.
		type MaxAuthorities: Get<u32>;

		/// Something that executes the payload
		type Executor: thea_primitives::TheaIncomingExecutor;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
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

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, thea_primitives::ValidatorSetId, ValueQuery>;

	/// Outgoing messages,
	/// first key: Nonce of the outgoing message
	#[pallet::storage]
	#[pallet::getter(fn outgoing_messages)]
	pub(super) type OutgoingMessages<T: Config> =
		StorageMap<_, Identity, u64, Message, OptionQuery>;

	/// Incoming messages,
	/// first key: Nonce of the incoming message
	#[pallet::storage]
	#[pallet::getter(fn incoming_messages)]
	pub(super) type IncomingMessages<T: Config> =
		StorageMap<_, Identity, u64, Message, OptionQuery>;

	/// Last processed nonce of this network
	#[pallet::storage]
	#[pallet::getter(fn outgoing_nonce)]
	pub(super) type OutgoingNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	/// Last processed nonce on native network
	#[pallet::storage]
	#[pallet::getter(fn incoming_nonce)]
	pub(super) type IncomingNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		TheaMessageExecuted { message: Message },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Unknown Error
		Unknown,
		/// Error executing thea message
		ErrorExecutingMessage,
		/// Wrong nonce provided
		MessageNonce,
		/// Error decoding validator set
		ErrorDecodingValidatorSet,
		/// Invalid Validator Set id
		InvalidValidatorSetId,
		/// Validator set is empty
		ValidatorSetEmpty,
		/// Cannot update with older nonce
		NonceIsAlreadyProcessed,
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
		/// Inserts a new authority set using sudo
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::insert_authorities(1))]
		#[transactional]
		pub fn insert_authorities(
			origin: OriginFor<T>,
			authorities: BoundedVec<T::TheaId, T::MaxAuthorities>,
			set_id: thea_primitives::ValidatorSetId,
		) -> DispatchResult {
			ensure_root(origin)?;
			<Authorities<T>>::insert(set_id, authorities);
			<ValidatorSetId<T>>::put(set_id);
			Ok(())
		}

		/// Handles the verified incoming message
		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::incoming_message())]
		#[transactional]
		pub fn incoming_message(
			origin: OriginFor<T>,
			payload: Message,
			_signatures: Vec<(u16, T::Signature)>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// Signature is already verified in validate_unsigned, no need to do it again

			let current_set_id = <ValidatorSetId<T>>::get();

			if !payload.is_key_change {
				// Normal Thea message
				T::Executor::execute_deposits(payload.network, payload.data.clone());
			} else {
				// Thea message related to key change
				match ValidatorSet::decode(&mut payload.data.as_ref()) {
					Err(_err) => return Err(Error::<T>::ErrorDecodingValidatorSet.into()),
					Ok(validator_set) => {
						ensure!(
							current_set_id.saturating_add(1) == validator_set.set_id,
							Error::<T>::InvalidValidatorSetId
						);
						<Authorities<T>>::insert(
							validator_set.set_id,
							BoundedVec::truncate_from(validator_set.validators),
						);
					},
				}
			}
			Self::deposit_event(Event::TheaMessageExecuted { message: payload.clone() });
			// We are checking if the validator set is changed, then we update it here too
			if current_set_id.saturating_add(1) == payload.validator_set_id {
				<ValidatorSetId<T>>::put(current_set_id.saturating_add(1));
			}
			<IncomingNonce<T>>::put(payload.nonce);
			<IncomingMessages<T>>::insert(payload.nonce, payload);
			Ok(())
		}

		/// A governance endpoint to update last processed nonce
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::update_incoming_nonce(1))]
		#[transactional]
		pub fn update_incoming_nonce(origin: OriginFor<T>, nonce: u64) -> DispatchResult {
			ensure_root(origin)?;
			<IncomingNonce<T>>::put(nonce);
			Ok(())
		}

		/// A governance endpoint to update outgoing nonces
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::update_outgoing_nonce(1))]
		#[transactional]
		pub fn update_outgoing_nonce(origin: OriginFor<T>, nonce: u64) -> DispatchResult {
			ensure_root(origin)?;
			<OutgoingNonce<T>>::put(nonce);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn validate_incoming_message(
		payload: &Message,
		signatures: &Vec<(u16, T::Signature)>,
	) -> TransactionValidity {
		// Check if this message can be processed next by checking its nonce
		let next_nonce = <IncomingNonce<T>>::get().saturating_add(1);

		if payload.nonce != next_nonce {
			log::error!(target:"thea","Next nonce: {:?}, incoming nonce: {:?}",next_nonce, payload.nonce);
			return InvalidTransaction::Custom(1).into()
		}

		if <ValidatorSetId<T>>::get() < payload.validator_set_id {
			log::error!(target:"thea","Future validator set: Stored: {:?}, Given: {:?}",<ValidatorSetId<T>>::get(), payload.validator_set_id);
			// Reject message from future validator sets
			return InvalidTransaction::Custom(2).into()
		}
		let authorities = <Authorities<T>>::get(payload.validator_set_id).to_vec();
		// Check for super majority
		const MAJORITY: u8 = 67;
		let p = Percent::from_percent(MAJORITY);
		let threshold = p * authorities.len();

		if signatures.len() < threshold {
			log::error!(target:"thea","Threshold: {:?}, Signs len: {:?}",threshold, signatures.len());
			return InvalidTransaction::Custom(3).into()
		}

		let encoded_payload = sp_io::hashing::sha2_256(&payload.encode());
		for (index, signature) in signatures {
			log::debug!(target:"thea", "Get auth of index: {:?}",index);
			match authorities.get(*index as usize) {
				None => return InvalidTransaction::Custom(4).into(),
				Some(auth) =>
					if !auth.verify(&encoded_payload, &((*signature).clone().into())) {
						log::debug!(target:"thea", "signature of index: {:?} -> {:?}, Failed",index,auth);
						return InvalidTransaction::Custom(5).into()
					},
			}
		}

		ValidTransaction::with_tag_prefix("thea")
			.and_provides(payload)
			.longevity(3)
			.propagate(true)
			.build()
	}

	/// Returns the current authority set
	pub fn get_current_authorities() -> Vec<T::TheaId> {
		let current_set_id = Self::validator_set_id();
		<Authorities<T>>::get(current_set_id).to_vec()
	}
}

impl<T: Config> thea_primitives::TheaOutgoingExecutor for Pallet<T> {
	fn execute_withdrawals(network: Network, data: Vec<u8>) -> DispatchResult {
		let authorities_len = <Authorities<T>>::get(Self::validator_set_id()).len();
		if authorities_len == 0 {
			return Err(Error::<T>::ValidatorSetEmpty.into())
		}
		let nonce = <OutgoingNonce<T>>::get();
		let payload = Message {
			block_no: frame_system::Pallet::<T>::current_block_number().saturated_into(),
			nonce: nonce.saturating_add(1),
			data,
			network,
			is_key_change: false,
			validator_set_id: Self::validator_set_id(),
		};
		// Update nonce
		<OutgoingNonce<T>>::put(payload.nonce);
		<OutgoingMessages<T>>::insert(payload.nonce, payload);

		Ok(())
	}
}
