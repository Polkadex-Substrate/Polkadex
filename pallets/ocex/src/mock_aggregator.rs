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

use crate::aggregator::AggregatorClient;
use crate::Config;
use lazy_static::lazy_static;
use orderbook_primitives::types::UserActionBatch;
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::AccountId;
use sp_core::crypto::AccountId32;
use std::sync::{Arc, Mutex};

lazy_static! {
	static ref SHARED_DATA: Arc<Mutex<Option<UserActionBatch<AccountId>>>> =
		Arc::new(Mutex::new(None));
}

impl<T: Config> AggregatorClient<T> {
	#[cfg(test)]
	pub fn get_user_action_batch(_id: u64) -> Option<UserActionBatch<T::AccountId>> {
		let data = SHARED_DATA.lock().unwrap();
		let data: Option<UserActionBatch<T::AccountId>> = if let Some(data) = data.clone() {
			let data = data.encode();
			Some(UserActionBatch::decode(&mut &data[..]).unwrap())
		} else {
			None
		};
		data
	}

	#[cfg(test)]
	pub fn send_request(
		_log_target: &str,
		_url: &str,
		_body: &str,
	) -> Result<Vec<u8>, &'static str> {
		Ok(Vec::new())
	}

	#[cfg(test)]
	pub fn mock_get_user_action_batch(user_action_batch: UserActionBatch<T::AccountId>) {
		let user_action_batch: UserActionBatch<AccountId32> =
			Decode::decode(&mut &user_action_batch.encode()[..]).unwrap();
		let mut data = SHARED_DATA.lock().unwrap();
		*data = Some(user_action_batch);
	}
}
