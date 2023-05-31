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

use log::warn;
use std::sync::Arc;

use crate::error::Error;
use orderbook_primitives::crypto::AuthorityId;
use sc_keystore::LocalKeystore;
use sp_core::Pair;

pub struct OrderbookKeyStore {
	keystore: Option<Arc<LocalKeystore>>,
}

impl OrderbookKeyStore {
	pub fn new(keystore: Option<Arc<LocalKeystore>>) -> Self {
		Self { keystore }
	}

	pub fn get_local_key(&self, active: &[AuthorityId]) -> Result<AuthorityId, Error> {
		match self.keystore.as_ref() {
			None => {
				warn!(target:"orderbook","📒 Keystore not available");
				return Err(Error::Keystore("📒 Keystore not available in this context".to_string()))
			},
			Some(keystore) =>
				for key in active {
					if let Some(local_pair) =
						keystore.key_pair::<orderbook_primitives::crypto::Pair>(key)?
					{
						return Ok(local_pair.public())
					}
				},
		}
		warn!(target:"orderbook","📒 No BLS key found");
		Err(Error::Keystore("📒 No BLS key found".to_string()))
	}

	pub fn sign(
		&self,
		public: &AuthorityId,
		message: &[u8],
	) -> Result<orderbook_primitives::crypto::AuthoritySignature, Error> {
		match self.keystore.as_ref() {
			None => {
				warn!(target:"orderbook","📒 Keystore not available");
				Err(Error::Keystore("📒 Keystore not available in this context".to_string()))
			},
			Some(keystore) => {
				match keystore.key_pair::<orderbook_primitives::crypto::Pair>(public)? {
					Some(local_pair) => Ok(local_pair.sign(message)),
					None => {
						warn!(target:"orderbook","📒 No BLS key found");
						Err(Error::Keystore("📒 No BLS key found".to_string()))
					},
				}
			},
		}
	}
}
