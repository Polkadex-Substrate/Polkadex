//! RPC API for Orderbook.

#![warn(missing_docs)]

use std::sync::Arc;

use codec::{Decode, Encode};
use futures::{channel::mpsc::UnboundedSender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::info;
use parking_lot::RwLock;
use reference_trie::ExtensionLayout;
use rust_decimal::Decimal;
use sc_rpc::SubscriptionTaskExecutor;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

use orderbook::DbRef;
use orderbook_primitives::{
	recovery::ObRecoveryState,
	types::{AccountAsset, ObMessage},
	ObApi,
};

#[derive(Debug, thiserror::Error)]
/// Top-level error type for the RPC handler
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

// Provides RPC methods for interacting with Orderbook.
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
	/// - self: a reference to the current object
	///
	/// # Return
	/// - RpcResult<String>: a Result containing serialized `ObRecoveryState`.
	#[method(name = "ob_getObRecoverState")]
	async fn get_orderbook_recovery_state(&self) -> RpcResult<String>;
}

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc<Runtime, Block, Client> {
	tx: UnboundedSender<ObMessage>,
	_executor: SubscriptionTaskExecutor,
	memory_db: DbRef,
	working_state_root: Arc<RwLock<[u8; 32]>>,
	runtime: Arc<Runtime>,
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Runtime, Block, Client> OrderbookRpc<Runtime, Block, Client>
where
	Block: BlockT,
	Runtime: Send + Sync + ProvideRuntimeApi<Block>,
	Runtime::Api: ObApi<Block>,
	Client: Send + Sync + HeaderBackend<Block>,
{
	/// Creates a new Orderbook Rpc handler instance.
	pub fn new(
		_executor: SubscriptionTaskExecutor,
		tx: UnboundedSender<ObMessage>,
		memory_db: DbRef,
		working_state_root: Arc<RwLock<[u8; 32]>>,
		runtime: Arc<Runtime>,
		client: Arc<Client>,
	) -> Self {
		Self {
			tx,
			_executor,
			memory_db,
			working_state_root,
			runtime,
			client,
			_marker: Default::default(),
		}
	}

	/// Returns the serialized offchain state based on the last finalized snapshot summary
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

		info!(target:"orderbook-rpc","Getting allowlisted asset ids");
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

		// Create existing DB, it will fail if root does not exist
		let trie: TrieDBMut<ExtensionLayout> =
			TrieDBMutBuilder::from_existing(&mut memory_db, &mut worker_state_root).build();

		let mut ob_recovery_state = ObRecoveryState::default();

		// Generate account info from existing DB
		let insert_balance = |trie: &TrieDBMut<ExtensionLayout>,
		                      ob_recovery_state: &mut ObRecoveryState,
		                      account_asset: &AccountAsset|
		 -> RpcResult<()> {
			if let Ok(data) = trie.get(&account_asset.encode()) {
				if let Some(data) = data {
					let account_balance = Decimal::decode(&mut &data[..]).map_err(|err| {
						JsonRpseeError::Custom(err.to_string() + "failed to decode decimal")
					})?;
					ob_recovery_state.balances.insert(account_asset.clone(), account_balance);
				}
			// Ignored none case as account may not have balance for asset
			} else {
				info!(target: "orderbook-rpc", "unable to fetch data for account: {:?}, asset: {:?}",&account_asset.main,&account_asset.asset);
				return Err(JsonRpseeError::Custom(
					"unable to fetch DB data for account".to_string(),
				))
			}
			Ok(())
		};
		info!(target:"orderbook-rpc","Loading balances from trie to result...");
		for (user_main_account, list_of_proxy_accounts) in all_register_accounts {
			for asset in allowlisted_asset_ids.clone() {
				let account_asset = AccountAsset::new(user_main_account.clone(), asset);
				insert_balance(&trie, &mut ob_recovery_state, &account_asset)?;
			}
			ob_recovery_state.account_ids.insert(user_main_account, list_of_proxy_accounts);
		}

		ob_recovery_state.snapshot_id = last_snapshot_summary.snapshot_id;
		ob_recovery_state.state_change_id = last_snapshot_summary.state_change_id;
		ob_recovery_state.worker_nonce = last_snapshot_summary.worker_nonce;
		ob_recovery_state.last_processed_block_number = last_snapshot_summary.last_processed_blk;

		info!(target:"orderbook-rpc","Serializing Orderbook snapshot state");
		let serialize_ob_recovery_state = serde_json::to_string(&ob_recovery_state)?;
		info!(target:"orderbook-rpc","Orderbook snapshot state exported");
		Ok(serialize_ob_recovery_state)
	}
}

#[async_trait]
impl<Runtime, Block, Client> OrderbookApiServer for OrderbookRpc<Runtime, Block, Client>
where
	Block: BlockT,
	Runtime: Send + Sync + ProvideRuntimeApi<Block> + 'static,
	Runtime::Api: ObApi<Block>,
	Client: Send + Sync + HeaderBackend<Block> + 'static,
{
	async fn submit_action(&self, message: ObMessage) -> RpcResult<()> {
		let mut tx = self.tx.clone();
		tx.send(message).await?;
		Ok(())
	}

	async fn get_orderbook_recovery_state(&self) -> RpcResult<String> {
		self.get_orderbook_recovery_state_inner().await
	}
}
