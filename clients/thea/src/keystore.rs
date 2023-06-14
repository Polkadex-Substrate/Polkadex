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
use sc_keystore::LocalKeystore;
use sp_core::Pair;
use std::sync::Arc;
use thea_primitives::crypto::{AuthorityId, AuthoritySignature};

/// Key store definition which holds keys and performs messages signing operations and accessor to
/// the public keys.
pub struct TheaKeyStore {
	keystore: Arc<LocalKeystore>,
}

impl TheaKeyStore {
	/// Constructor.
	///
	/// # Parameters
	///
	/// * `keystore`: Local keystore from the keystore container.
	pub fn new(keystore: Arc<LocalKeystore>) -> Self {
		Self { keystore }
	}

	/// Accessor to the BLS public key by identity of Thea authority.
	///
	/// # Parameters
	///
	/// * `active`: Identifier of the Thea authority.
	pub fn get_local_key(&self, active: &[AuthorityId]) -> Result<AuthorityId, Error> {
		for key in active {
			if let Some(local_pair) =
				self.keystore.key_pair::<thea_primitives::crypto::Pair>(key)?
			{
				return Ok(local_pair.public())
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
	pub fn sign(&self, public: &AuthorityId, message: &[u8]) -> Result<AuthoritySignature, Error> {
		match self.keystore.key_pair::<thea_primitives::crypto::Pair>(public)? {
			Some(local_pair) => Ok(local_pair.sign(message)),
			None => {
				warn!(target:"thea","🌉 No BLS key found");
				Err(Error::Keystore("🌉 No BLS key found".to_string()))
			},
		}
	}
}
