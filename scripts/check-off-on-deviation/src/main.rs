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

//! Maintains an `JsonrpseeApiClient` client and calls necessary `JsonrpseeApiClient` requests
//! methods when required by different modules. The client is initialized in main module.

use std::collections::BTreeMap;
use jsonrpsee::{
	core::client::ClientT,
	rpc_params,
	ws_client::{WsClient, WsClientBuilder},
};
use orderbook_primitives::recovery::ObRecoveryState;

use std::sync::Arc;

/// A Jsonrpsee client for blockchain used by block relayer to get required data from chain.
pub struct JsonrpseeApiClient {
	pub client: Arc<WsClient>,
	pub url: String,
}

impl JsonrpseeApiClient {
	/// A WebSocket client is created using the WsClientBuilder and the resulting client is stored
	/// in an Arc (atomic reference counted) pointer to allow for shared ownership across threads.
	pub async fn new(blockchain_url: &str) -> anyhow::Result<Self> {
		let client = WsClientBuilder::default().build(blockchain_url).await?;
		Ok(Self {
			client: Arc::new(client),
			url: blockchain_url.to_string(),
		})
	}

	/// Retrieves the current OB recovery state from the engine.
	/// Returns the recovery state as an `ObRecoveryState` object if successful.
	///
	/// Returns
	///
	/// `anyhow::Result<ObRecoveryState>`: The state at which engine should be recovered
	pub async fn get_recovery_state(&mut self) -> anyhow::Result<ObRecoveryState> {
		// Start the connection again
		self.client = Arc::new(WsClientBuilder::default().build(self.url.as_str()).await?);

		// Retrieve the recovery state from the engine
		Ok(self
			.client
			.request::<ObRecoveryState, _>("ob_getRecoverState", rpc_params![])
			.await?)
	}
}

#[tokio::test]
pub async fn test_parse_recovery() {
	let mut client = JsonrpseeApiClient::new("wss://fullnode.polkadex.trade:443")
		.await
		.unwrap();
	client.get_recovery_state().await.unwrap();
}


#[tokio::main]
async fn main() {
	let mut client = JsonrpseeApiClient::new("ws://localhost:9944").await.unwrap();

	let ob_state = client.get_recovery_state().await.unwrap();

	let mut assets = BTreeMap::new();

	for (acc_asset,total) in ob_state.balances{
		assets.entry(acc_asset.asset).and_modify(|amt | {
			*amt +=total
		}).or_insert(total);
	}
	println!("Offchain -balance: {:?}",assets);
}