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
use ocex_primitives::SigningError;
use sp_runtime::RuntimeAppPublic;
use sp_std::vec::Vec;

impl<T: Config> Pallet<T> {
	pub fn submit_approve_enclave_report(
		// TODO: @Ivan, please configure the params accordingly
		approver: T::OCEXId,
		signature: <T::OCEXId as RuntimeAppPublic>::Signature,
		report: Vec<u8>,
	) -> Result<(), SigningError> {
		let call = Call::approve_enclave_report { approver, signature, report };
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| SigningError::OffchainUnsignedTxError)
	}
}
