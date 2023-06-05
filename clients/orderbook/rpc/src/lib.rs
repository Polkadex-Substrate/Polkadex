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

//! Defines RPC abstraction and concrete implementation required to communicate with the
//! `Orderbook`.

#![warn(missing_docs)]

use std::sync::Arc;

use codec::{Decode, Encode};
use futures::{channel::mpsc::UnboundedSender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::{error, info, warn};
use memory_db::{HashKey, MemoryDB};
use parking_lot::RwLock;
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::Decimal;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_blockchain::HeaderBackend;
use sp_core::offchain::OffchainStorage;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};
use orderbook::{snapshot::SnapshotStore, DbRef};
use orderbook_primitives::{
	recovery::ObRecoveryState,
	types::{AccountAsset, ObMessage},
	ObApi, ORDERBOOK_STATE_CHUNK_PREFIX,
};

/// Top-level error type for the RPC handler.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// The Orderbook RPC endpoint is not ready.
	#[error("Orderbook RPC endpoint not ready")]
	EndpointNotReady,
	/// The Orderbook RPC background task failed to spawn.
	#[error("Orderbook RPC background task failed to spawn")]
	RpcTaskFailure(#[from] SpawnError),
}

/// The error codes returned by jsonrpc.
pub enum ErrorCode {
	/// Returned when Orderbook RPC endpoint is not ready.
	NotReady = 1,
	/// Returned on Orderbook RPC background task failure.
	TaskFailure = 2,
}

impl From<Error> for ErrorCode {
	fn from(error: Error) -> Self {
		match error {
			Error::EndpointNotReady => ErrorCode::NotReady,
			Error::RpcTaskFailure(_) => ErrorCode::TaskFailure,
		}
	}
}

impl From<Error> for JsonRpseeError {
	fn from(error: Error) -> Self {
		let message = error.to_string();
		let code = ErrorCode::from(error);
		JsonRpseeError::Call(CallError::Custom(ErrorObject::owned(
			code as i32,
			message,
			None::<()>,
		)))
	}
}

/// RPC abstraction for interacting with Orderbook.
#[rpc(client, server)]
pub trait OrderbookApi {
	/// Returns hash of the latest Orderbook finalized block as seen by this client.
	///
	/// The latest Orderbook block might not be available if the Orderbook gadget is not running
	/// in the network or if the client is still initializing or syncing with the network.
	/// In such case an error would be returned.
	#[method(name = "ob_submitAction")]
	async fn submit_action(&self, action: ObMessage) -> RpcResult<()>;

	/// Returns the state of the orderbook that will help engine to recover.
	///
	/// # Parameters
	///
	/// * `self`: A reference to the current object.
	///
	/// # Returns
	///
	/// * `RpcResult<String>`: A Result containing serialized `ObRecoveryState`.
	#[method(name = "ob_getObRecoverState")]
	async fn get_orderbook_recovery_state(&self) -> RpcResult<String>;

	/// Returns the state of the orderbook for a specific snapshot
	///
	/// # Parameters
	/// - self: a reference to the current object
	/// - snapshot_id: id of the requested snapshot
	///
	/// # Return
	/// - RpcResult<String>: a Result containing serialized `ObRecoveryState`.
	#[method(name = "ob_getObRecoverStateFromStorage")]
	async fn get_orderbook_recovery_state_from_storage(
		&self,
		snapshot_id: u64,
	) -> RpcResult<String>;
}

#[async_trait]
impl<Block, Client, Backend, Runtime> OrderbookApiServer
	for OrderbookRpc<Block, Client, Backend, Runtime>
where
	Block: BlockT,
	Runtime: Send + Sync + ProvideRuntimeApi<Block> + 'static,
	Runtime::Api: ObApi<Block>,
	Client: Send + Sync + HeaderBackend<Block> + 'static,
	Backend: Send + Sync + sc_client_api::Backend<Block> + 'static,
{
	async fn submit_action(&self, message: ObMessage) -> RpcResult<()> {
		let mut tx = self.tx.clone();
		tx.send(message).await?;
		Ok(())
	}

	async fn get_orderbook_recovery_state(&self) -> RpcResult<String> {
		self.get_orderbook_recovery_state_inner().await
	}

	async fn get_orderbook_recovery_state_from_storage(
		&self,
		snapshot_id: u64,
	) -> RpcResult<String> {
		self.get_orderbook_recovery_state_from_storage_inner(snapshot_id).await
	}
}

/// Orderbook specific RPC dependencies
pub struct OrderbookDeps<Backend, Client, Runtime> {
	/// Client Backend
	pub backend: Arc<Backend>,
	/// Client
	pub client: Arc<Client>,
	/// Runtime
	pub runtime: Arc<Runtime>,
	/// Channel for sending ob messages to worker
	pub rpc_channel: UnboundedSender<ObMessage>,
	/// memory db
	#[allow(clippy::type_complexity)]
	pub memory_db: Arc<RwLock<MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>>>,
	/// working_state_root
	pub working_state_root: Arc<RwLock<[u8; 32]>>,
}

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc<Block, Client, Backend, Runtime> {
	tx: UnboundedSender<ObMessage>,
	memory_db: DbRef,
	working_state_root: Arc<RwLock<[u8; 32]>>,
	runtime: Arc<Runtime>,
	client: Arc<Client>,
	backend: Arc<Backend>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Block, Client, Backend, Runtime> OrderbookRpc<Block, Client, Backend, Runtime>
where
	Block: BlockT,
	Runtime: Send + Sync + ProvideRuntimeApi<Block>,
	Runtime::Api: ObApi<Block>,
	Client: Send + Sync + HeaderBackend<Block>,
	Backend: sc_client_api::Backend<Block>,
{
	/// Creates a new Orderbook Rpc handler instance.
	///
	/// # Parameters
	///
	/// * `tx`: Channel for sending messages to the worker.
	/// * `memory_db`: Reference to the MemoryDB to create in-memory trie representation from it.
	/// * `working_state_root`: Working state root key in MemoryDB from which trie representation
	///   will be created.
	/// * `runtime`: Something that provides a runtime api.
	/// * `client`: Blockchain database header backend concrete implementation.
	pub fn new(deps: OrderbookDeps<Backend, Client, Runtime>) -> Self {
		Self {
			tx: deps.rpc_channel,
			memory_db: deps.memory_db,
			working_state_root: deps.working_state_root,
			runtime: deps.runtime.clone(),
			client: deps.client.clone(),
			backend: deps.backend,
			_marker: Default::default(),
		}
	}

	/// Returns the serialized offchain state based on the last finalized snapshot summary.
	pub async fn get_orderbook_recovery_state_inner(&self) -> RpcResult<String> {
		// get snapshot summary
		let last_snapshot_summary = self
			.runtime
			.runtime_api()
			.get_latest_snapshot(&BlockId::number(self.client.info().finalized_number))
			.map_err(|err| {
				JsonRpseeError::Custom(err.to_string() + "failed to get snapshot summary")
			})?;

		let memory_db_guard = self.memory_db.read();
		let mut memory_db = memory_db_guard.clone();
		let worker_state_root_guard = self.working_state_root.read();
		let mut worker_state_root = *worker_state_root_guard;
		info!(target:"orderbook-rpc","Getting all registered accounts at last finalized snapshot");
		// get all accounts
		let all_register_accounts = self
			.runtime
			.runtime_api()
			.get_all_accounts_and_proxies(&BlockId::number(
				last_snapshot_summary.last_processed_blk.saturated_into(),
			))
			.map_err(|err| JsonRpseeError::Custom(err.to_string() + "failed to get accounts"))?;

		info!(target:"orderbook-rpc","main accounts found: {:?}, Getting last finalized snapshot summary",all_register_accounts.len());

		// Get all allow listed AssetIds
		let allowlisted_asset_ids = self
			.runtime
			.runtime_api()
			.get_allowlisted_assets(&BlockId::number(
				last_snapshot_summary.last_processed_blk.saturated_into(),
			))
			.map_err(|err| {
				JsonRpseeError::Custom(err.to_string() + "failed to get allow listed asset ids")
			})?;
		info!(target:"orderbook-rpc","Getting allowlisted asset ids: {:?}", allowlisted_asset_ids);
		// Create existing DB, it will fail if root does not exist
		let trie: TrieDBMut<ExtensionLayout> =
			TrieDBMutBuilder::from_existing(&mut memory_db, &mut worker_state_root).build();

		let mut ob_recovery_state = ObRecoveryState::default();

		// Generate account info from existing DB
		info!(target:"orderbook-rpc","Loading balances from trie to result...");
		for (user_main_account, list_of_proxy_accounts) in all_register_accounts {
			for asset in allowlisted_asset_ids.clone() {
				let account_asset = AccountAsset::new(user_main_account.clone(), asset);
				self.insert_balance(&trie, &mut ob_recovery_state, &account_asset)?;
			}
			ob_recovery_state.account_ids.insert(user_main_account, list_of_proxy_accounts);
		}

		ob_recovery_state.snapshot_id = last_snapshot_summary.snapshot_id;
		ob_recovery_state.state_change_id = last_snapshot_summary.state_change_id;
		ob_recovery_state.worker_nonce = last_snapshot_summary.worker_nonce;
		ob_recovery_state.last_processed_block_number = last_snapshot_summary.last_processed_blk;
		ob_recovery_state.state_version = last_snapshot_summary.state_version;

		info!(target:"orderbook-rpc","Serializing Orderbook snapshot state");
		let serialize_ob_recovery_state = serde_json::to_string(&ob_recovery_state)?;
		info!(target:"orderbook-rpc","Orderbook snapshot state exported");
		Ok(serialize_ob_recovery_state)
	}

	async fn get_orderbook_recovery_state_from_storage_inner(
		&self,
		snapshot_id: u64,
	) -> RpcResult<String> {
		let offchain_storage = self
			.backend
			.offchain_storage()
			.ok_or(JsonRpseeError::Custom("Unable to access offchain storage".parse().unwrap()))?;

		let summary = self
			.runtime
			.runtime_api()
			.get_snapshot_by_id(&BlockId::number(self.client.info().finalized_number), snapshot_id)
			.map_err(|err| {
				JsonRpseeError::Custom(err.to_string() + "failed to get snapshot summary")
			})?
			.ok_or(JsonRpseeError::Custom("Snapshot not availabe in runtime".parse().unwrap()))?;

		info!(target:"orderbook-rpc","Summary Loaded: {:?}",summary);

		let mut data = Vec::new();

		for chunk in summary.state_chunk_hashes {
			let mut chunk_data = offchain_storage
				.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk.0.as_ref())
				.ok_or(JsonRpseeError::Custom(format!("Chunk not found: {chunk:?}")))?;
			info!(target:"orderbook-rpc","Chunk Loaded: {:?}",chunk);
			data.append(&mut chunk_data);
		}

		let mut worker_state_root = summary.state_root.0;

		let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();

		let store: SnapshotStore = serde_json::from_slice(&data)?;
		memory_db.load_from(store.convert_to_hashmap());

		// get all accounts
		let all_register_accounts = self
			.runtime
			.runtime_api()
			.get_all_accounts_and_proxies(&BlockId::number(self.client.info().finalized_number))
			.map_err(|err| JsonRpseeError::Custom(err.to_string() + "failed to get accounts"))?;

		info!(target:"orderbook-rpc","main accounts found: {:?}, Getting last finalized snapshot summary",all_register_accounts.len());

		// Get all allow listed AssetIds
		let allowlisted_asset_ids = self
			.runtime
			.runtime_api()
			.get_allowlisted_assets(&BlockId::number(self.client.info().finalized_number))
			.map_err(|err| {
				JsonRpseeError::Custom(err.to_string() + "failed to get allow listed asset ids")
			})?;
		info!(target:"orderbook-rpc","Getting allowlisted asset ids: {:?}", allowlisted_asset_ids);

		// Create existing DB, it will fail if root does not exist
		let mut trie: TrieDBMut<ExtensionLayout> =
			TrieDBMutBuilder::from_existing(&mut memory_db, &mut worker_state_root).build();

		info!(target:"orderbook-rpc","Trie loaded, empty: {:?}, Root hash: 0x{}",trie.is_empty(), hex::encode(trie.root()));

		let mut ob_recovery_state = ObRecoveryState::default();

		// Generate account info from existing DB
		info!(target:"orderbook-rpc","Loading balances from trie to result...");
		for (user_main_account, list_of_proxy_accounts) in all_register_accounts {
			for asset in allowlisted_asset_ids.clone() {
				let account_asset = AccountAsset::new(user_main_account.clone(), asset);
				self.insert_balance(&trie, &mut ob_recovery_state, &account_asset)?;
			}
			// Check if main account exists in the trie
			if trie
				.contains(&user_main_account.encode())
				.map_err(|err| JsonRpseeError::Custom(format!("Error accessing trie: {err:?}")))?
			{
				ob_recovery_state.account_ids.insert(user_main_account, list_of_proxy_accounts);
			} else {
				warn!(target:"orderbook-rpc","Main account not found: {:?}",user_main_account);
			}
		}

		ob_recovery_state.snapshot_id = summary.snapshot_id;
		ob_recovery_state.state_change_id = summary.state_change_id;
		ob_recovery_state.worker_nonce = summary.worker_nonce;
		ob_recovery_state.last_processed_block_number = summary.last_processed_blk;
		ob_recovery_state.state_version = summary.state_version;

		info!(target:"orderbook-rpc","Serializing Orderbook snapshot state");
		let serialize_ob_recovery_state = serde_json::to_string(&ob_recovery_state)?;
		info!(target:"orderbook-rpc","Orderbook snapshot state exported");
		Ok(serialize_ob_recovery_state)
	}

	/// Inserts balances to the trie
	pub fn insert_balance(
		&self,
		trie: &TrieDBMut<ExtensionLayout>,
		ob_recovery_state: &mut ObRecoveryState,
		account_asset: &AccountAsset,
	) -> RpcResult<()> {
		if let Ok(data) = trie.get(&account_asset.encode()) {
			if let Some(data) = data {
				let account_balance = Decimal::decode(&mut &data[..]).map_err(|err| {
					JsonRpseeError::Custom(err.to_string() + "failed to decode decimal")
				})?;
				ob_recovery_state.balances.insert(account_asset.clone(), account_balance);
			} else {
				log::warn!(target:"orderbook-rpc","No balance found for: {account_asset:?}");
			}
		// Ignored none case as account may not have balance for asset
		} else {
			error!(target: "orderbook-rpc", "unable to fetch data for account: {:?}, asset: {:?}",&account_asset.main,&account_asset.asset);
			return Err(JsonRpseeError::Custom(format!(
				"unable to fetch data for account: {:?}, asset: {:?}",
				&account_asset.main, &account_asset.asset
			)))
		}
		Ok(())
	}
}
