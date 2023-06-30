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

//! Local keystore implementation.

use crate::error::Error;
use log::warn;
use sp_core::bls381::{Public, Signature};
use sp_keystore::KeystorePtr;
pub use thea_primitives::KEY_TYPE as TheaKeyType;

/// Key store definition which holds keys and performs messages signing operations and accessor to
/// the public keys.
pub struct TheaKeyStore {
	keystore: KeystorePtr,
}

impl TheaKeyStore {
	/// Constructor.
	///
	/// # Parameters
	///
	/// * `keystore`: Local keystore from the keystore container.
	pub fn new(keystore: KeystorePtr) -> Self {
		Self { keystore }
	}

	/// Accessor to the BLS public key by identity of Thea authority.
	///
	/// # Parameters
	///
	/// * `active`: Identifier of the Thea authority.
	pub fn get_local_key(&self, active: &[Public]) -> Result<Public, Error> {
		for key in active {
			if let Some(local) = self
				.keystore
				.bls381_public_keys(TheaKeyType)
				.iter()
				.find(|stored| stored.eq(&key))
			{
				return Ok(*key)
			}
		}
		warn!(target:"thea","🌉 No BLS key found");
		Err(Error::Keystore("🌉 No BLS key found".to_string()))
	}

	/// Signs provided message with stored BLS key related to the Thea authority.
	///
	/// # Parameters
	///
	/// * `public`: Identifier of the Thea authority using BLS as its crypto.
	/// * `message`: Message to sign.
	pub fn sign(&self, public: &Public, message: &[u8]) -> Result<Signature, Error> {
		self.keystore.bls381_sign(TheaKeyType, public, message)?.ok_or_else(|| {
			warn!(target:"thea","🌉 No BLS key found for signing!");
			Error::Keystore("🌉 No BLS key found".to_string())
		})
	}
}
