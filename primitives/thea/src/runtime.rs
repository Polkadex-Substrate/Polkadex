// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
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

use sp_runtime_interface::runtime_interface;

/// Verifies the signature with given public key and message.
///
/// Note message is not hashed for verify_ecdsa and is prehashed for verify_ecdsa_prehashed
#[runtime_interface]
pub trait Crypto {
	fn verify_ecdsa(
		signature: &sp_core::ecdsa::Signature,
		public_key: &sp_core::ecdsa::Public,
		message: &[u8; 32],
	) -> bool {
		if let Some(pubk) = signature.recover(message) {
			&pubk == public_key
		} else {
			false
		}
		// let message = libsecp256k1::Message::parse(message);
		// let sig: (libsecp256k1::Signature, libsecp256k1::RecoveryId) = match signature.try_into()
		// { 	Ok(x) => x,
		// 	_ => return false,
		// };
		// match libsecp256k1::recover(&message, &sig.0, &sig.1) {
		// 	Ok(actual) => public_key.0[..] == actual.serialize_compressed()[..],
		// 	_ => false,
		// }
	}
	fn verify_ecdsa_prehashed(
		signature: &sp_core::ecdsa::Signature,
		public_key: &sp_core::ecdsa::Public,
		message: &[u8; 32],
	) -> bool {
		if let Some(pubk) = signature.recover_prehashed(message) {
			&pubk == public_key
		} else {
			false
		}
	}
}
