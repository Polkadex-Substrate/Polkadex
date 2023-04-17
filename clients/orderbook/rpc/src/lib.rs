//! RPC API for Orderbook.

#![warn(missing_docs)]

use sc_rpc::SubscriptionTaskExecutor;

use codec::{Decode, Encode};
use futures::{channel::mpsc::UnboundedSender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::info;
use orderbook::DbRef;
use orderbook_primitives::{
	types::{AccountAsset, ObMessage, ObRecoveryState},
	ObApi,
};
use parking_lot::RwLock;
use polkadex_primitives::BlockNumber;
use reference_trie::ExtensionLayout;
use rust_decimal::Decimal;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

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
	/// - RpcResult<Vec<u8>>: a Result containing serialized `ObRecoveryState`.
	#[method(name = "ob_getObRecoverState")]
	async fn get_orderbook_recovery_state(&self) -> RpcResult<Vec<u8>>;
}

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc<Client, Block> {
	tx: UnboundedSender<ObMessage>,
	_executor: SubscriptionTaskExecutor,
	last_successful_block_number_snapshot_created: Arc<RwLock<BlockNumber>>,
	memory_db: DbRef,
	working_state_root: Arc<RwLock<[u8; 32]>>,
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> OrderbookRpc<Client, Block> {
	/// Creates a new Orderbook Rpc handler instance.
	pub fn new(
		_executor: SubscriptionTaskExecutor,
		tx: UnboundedSender<ObMessage>,
		last_successful_block_number_snapshot_created: Arc<RwLock<BlockNumber>>,
		memory_db: DbRef,
		working_state_root: Arc<RwLock<[u8; 32]>>,
		client: Arc<Client>,
	) -> Self {
		Self {
			tx,
			_executor,
			last_successful_block_number_snapshot_created,
			memory_db,
			working_state_root,
			client,
			_marker: Default::default(),
		}
	}
}

#[async_trait]
impl<Client, Block> OrderbookApiServer for OrderbookRpc<Client, Block>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: ObApi<Block>,
{
	async fn submit_action(&self, message: ObMessage) -> RpcResult<()> {
		let mut tx = self.tx.clone();
		tx.send(message).await?;
		Ok(())
	}

	async fn get_orderbook_recovery_state(&self) -> RpcResult<Vec<u8>> {
		let last_finalized_block_guard = self.last_successful_block_number_snapshot_created.read();
		let last_finalized_block = *last_finalized_block_guard;

		let memory_db_guard = self.memory_db.read();
		let mut memory_db = memory_db_guard.clone();
		let worker_state_root_guard = self.working_state_root.read();
		let mut worker_state_root = *worker_state_root_guard;

		// get all accounts
		let all_register_accounts = self
			.client
			.runtime_api()
			.get_all_accounts_and_proxies(&BlockId::number(last_finalized_block.saturated_into()))
			.map_err(|err| JsonRpseeError::Custom(err.to_string() + "failed to get accounts"))?;

		// get snapshot summary
		let last_snapshot_summary = self
			.client
			.runtime_api()
			.get_latest_snapshot(&BlockId::number(last_finalized_block.saturated_into()))
			.map_err(|err| {
				JsonRpseeError::Custom(err.to_string() + "failed to get snapshot summary")
			})?;

		// Get all allow listed AssetIds
		let allowlisted_asset_ids = self
			.client
			.runtime_api()
			.get_allowlisted_assets(&BlockId::number(last_finalized_block.saturated_into()))
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

		for (user_main_account, list_of_proxy_accounts) in all_register_accounts {
			for asset in allowlisted_asset_ids.clone() {
				let account_asset = AccountAsset::new(user_main_account.clone(), asset);
				insert_balance(&trie, &mut ob_recovery_state, &account_asset)?;
			}
			ob_recovery_state.account_ids.insert(user_main_account, list_of_proxy_accounts);
		}

		ob_recovery_state.snapshot_id = last_snapshot_summary.snapshot_id;
		ob_recovery_state.state_change_id = last_snapshot_summary.worker_nonce;

		let serialize_ob_recovery_state = serde_json::to_vec(&ob_recovery_state)?;
		Ok(serialize_ob_recovery_state)
	}
}
