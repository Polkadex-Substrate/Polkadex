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

use jsonrpc_core::Result as RpcResult;
use jsonrpc_derive::rpc;
use std::sync::{mpsc::Receiver, Arc, Mutex};
use thea_client::worker::RoundInfo;

#[rpc]
pub trait TheaRpcApi {
	type Metadata;

	#[rpc(name = "theaInfo")]
	fn thea_info(&self) -> RpcResult<RoundInfo>;
}

pub struct TheaRpcApiHandler {
	cache: Mutex<RoundInfo>,
	// this wrapping is required by rpc boundaries
	updater: Arc<Mutex<Receiver<RoundInfo>>>,
}

impl TheaRpcApiHandler {
	pub fn new(updater: Arc<Mutex<Receiver<RoundInfo>>>) -> Self {
		Self { updater, cache: Mutex::new(RoundInfo::default()) }
	}
}

impl TheaRpcApi for TheaRpcApiHandler {
	type Metadata = sc_rpc::Metadata;

	fn thea_info(&self) -> RpcResult<RoundInfo> {
		// read latest from the channel
		if let Ok(upd_ref) = self.updater.lock() {
			if let Ok(mut inner) = self.cache.lock() {
				// exhausting sent updates if any to get latest state
				while let Ok(update) = upd_ref.recv_timeout(std::time::Duration::from_millis(50)) {
					*inner = update;
				}
			}
		}

		// send cached data
		Ok(self.cache.lock().map_err(|_| jsonrpc_core::Error::internal_error())?.clone())
	}
}
