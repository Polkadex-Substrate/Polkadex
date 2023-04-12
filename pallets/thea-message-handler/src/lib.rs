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
	types::Message, AuthorityIndex, Network, ValidatorSet, GENESIS_AUTHORITY_SET_ID, NATIVE_NETWORK,
};

#[frame_support::pallet]
pub mod pallet {
	use frame_support::transactional;
	use sp_runtime::Saturating;
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
	/// first key: Block number of foreign chain
	#[pallet::storage]
	#[pallet::getter(fn outgoing_messages)]
	pub(super) type OutgoingMessages<T: Config> =
		StorageMap<_, Identity, T::BlockNumber, Message, OptionQuery>;

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
		TheaMessageQueued { message: Message },
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

			let last_nonce = <IncomingNonce<T>>::get();
			if last_nonce != payload.nonce.saturating_add(1) {
				return Err(Error::<T>::MessageNonce.into())
			}
			let current_set_id = <ValidatorSetId<T>>::get();

			if !payload.is_key_change {
				// Normal Thea message
				if let Err(_) =
					T::Executor::execute_deposits(payload.network, payload.data.clone())
				{
					return Err(Error::<T>::ErrorExecutingMessage.into())
				}
			} else {
				// Thea message related to key change
				match ValidatorSet::decode(&mut &payload.data[..]) {
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
			// We are checking if the validator set is changed, then we update it here too
			if current_set_id.saturating_add(1) == payload.validator_set_id {
				<ValidatorSetId<T>>::put(current_set_id.saturating_add(1));
			}

			<IncomingNonce<T>>::put(payload.nonce);
			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	fn validate_incoming_message(
		bitmap: &Vec<u128>,
		payload: &Message,
		signature: &T::Signature,
	) -> TransactionValidity {
		// TODO: Implement aggregate signature verification using bls_primitives library.
		// Check message nonce is in order
		// Take the validator set in correct order
		// Verify aggregate signature with bitmap

		ValidTransaction::with_tag_prefix("thea")
			.and_provides([signature])
			.longevity(3)
			.propagate(true)
			.build()
	}
}

impl<T: Config> thea_primitives::TheaOutgoingExecutor for Pallet<T> {
	fn execute_withdrawals(network: Network, data: Vec<u8>) -> Result<(), ()> {
		// Only native networks are allowed in foreign chains
		if network != NATIVE_NETWORK {
			return Err(())
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
		<OutgoingMessages<T>>::insert(payload.block_no.saturated_into::<T::BlockNumber>(), payload);

		Ok(())
	}
}
