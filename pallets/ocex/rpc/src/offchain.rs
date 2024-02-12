// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

//! # Offchain Storage Adapter
//! This module provides an adapter to access offchain storage.
//! This adapter is used by `function_handler` to access offchain storage.

use parity_scale_codec::Encode;
use sp_core::offchain::{storage::OffchainDb, DbExternalities, OffchainStorage, StorageKind};

pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";

/// Adapter to Access OCEX Offchain Storage
pub struct OffchainStorageAdapter<T: OffchainStorage> {
	storage: OffchainDb<T>,
}

impl<T: OffchainStorage> OffchainStorageAdapter<T> {
	/// Create a new `OffchainStorageAdapter` instance.
	/// # Parameters
	/// * `storage`: Offchain storage
	/// # Returns
	/// * `OffchainStorageAdapter`: A new `OffchainStorageAdapter` instance.
	pub fn new(storage: OffchainDb<T>) -> Self {
		Self { storage }
	}

	/// Acquire offchain lock
	/// # Parameters
	/// * `tries`: Number of tries to acquire lock
	/// # Returns
	/// * `bool`: True if lock is acquired else false
	pub async fn acquire_offchain_lock(&mut self, tries: u8) -> bool {
		let old_value = Encode::encode(&false);
		let new_value = Encode::encode(&true);
		for _ in 0..tries {
			if self.storage.local_storage_compare_and_set(
				StorageKind::PERSISTENT,
				&WORKER_STATUS,
				Some(&old_value),
				&new_value,
			) {
				return true;
			}
			// Wait for 1 sec
			tokio::time::sleep(std::time::Duration::from_secs(1)).await;
		}
		false
	}
}

impl<T: OffchainStorage> Drop for OffchainStorageAdapter<T> {
	fn drop(&mut self) {
		let encoded_value = Encode::encode(&false);
		self.storage
			.local_storage_set(StorageKind::PERSISTENT, &WORKER_STATUS, &encoded_value);
	}
}
