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

use crate::{Call, Config, Pallet};
use frame_system::offchain::SubmitTransaction;
use sp_runtime::RuntimeAppPublic;
use thea_primitives::{
	keygen::{KeygenRound, OfflineStageRound, SigningSessionPayload, TheaPayload, OffenseReport},
	payload::SignedTheaPayload,
	AuthorityIndex, SigningError, AuthorityId
};

impl<T: Config> Pallet<T> {
	pub fn submit_keygen_message_api(
		payload: TheaPayload<
			T::TheaId,
			KeygenRound,
			thea_primitives::MsgLimit,
			thea_primitives::MsgVecLimit,
		>,
		signature: <T::TheaId as RuntimeAppPublic>::Signature,
		rng: u64,
	) -> Result<(), SigningError> {
		let call = Call::submit_keygen_message { payload, signature, rng };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}

	pub fn submit_clear_keygen_api(
		auth_idx: AuthorityIndex,
		signature: <T::TheaId as RuntimeAppPublic>::Signature,
		rng: u64,
	) -> Result<(), SigningError> {
		let call = Call::clean_keygen_messages { auth_idx, signature, rng };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}

	pub fn submit_offline_message_api(
		payload: TheaPayload<
			T::TheaId,
			OfflineStageRound,
			thea_primitives::MsgLimit,
			thea_primitives::MsgVecLimit,
		>,
		signature: <T::TheaId as RuntimeAppPublic>::Signature,
		rng: u64,
		payload_array: &[u8; 32],
	) -> Result<(), SigningError> {
		let call =
			Call::submit_offline_message { payload, payload_array: *payload_array, signature, rng };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}

	pub fn submit_signing_message_api(
		at: T::BlockNumber,
		payload: SigningSessionPayload<
			T::TheaId,
			thea_primitives::PartialSignatureLimit,
			thea_primitives::PartialSignatureVecLimit,
		>,
		signature: <T::TheaId as RuntimeAppPublic>::Signature,
		rng: u64,
	) -> Result<(), SigningError> {
		let call = Call::submit_signing_message { at, payload, signature, rng };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}

	pub fn submit_signed_payload_api(
		payload: SignedTheaPayload,
		rng: u64,
	) -> Result<(), SigningError> {
		let call = Call::submit_signed_payload { payload, rng };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}

	pub fn register_offence_api(
		signature: <T::TheaId as RuntimeAppPublic>::Signature, 
		offence: OffenseReport<T::AccountId>,
	) -> Result<(), SigningError> {
		let call = Call::register_offense{ signature, offence };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}
}
