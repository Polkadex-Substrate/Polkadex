//! RPC API for Orderbook.

#![warn(missing_docs)]

use sc_rpc::SubscriptionTaskExecutor;

use futures::{channel::mpsc::UnboundedSender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult, __reexports::serde_json},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::warn;
use orderbook_primitives::types::{ObMessage, ObRecoveryState};
use parking_lot::RwLock;
use polkadex_primitives::BlockNumber;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;

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
	#[method(name = "ob_getObRecoverState")]
	async fn get_orderbook_recovery_state(&self) -> RpcResult<Vec<u8>>;
}

use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::ObApi;
use reference_trie::{ExtensionLayout, RefHasher};

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc<Client, Block> {
	tx: UnboundedSender<ObMessage>,
	_executor: SubscriptionTaskExecutor,
	last_successful_block_no_snapshot_created: Arc<RwLock<BlockNumber>>,
	memory_db: Arc<RwLock<MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>>>,
	working_state_root: Arc<RwLock<[u8; 32]>>,
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> OrderbookRpc<Client, Block> {
	/// Creates a new Orderbook Rpc handler instance.
	pub fn new(
		_executor: SubscriptionTaskExecutor,
		tx: UnboundedSender<ObMessage>,
		last_successful_block_no_snapshot_created: Arc<RwLock<BlockNumber>>,
		memory_db: Arc<RwLock<MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>>>,
		working_state_root: Arc<RwLock<[u8; 32]>>,
		client: Arc<Client>,
	) -> Self {
		Self {
			tx,
			_executor,
			last_successful_block_no_snapshot_created,
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
		// Snapshot generation logic will fix it

		let last_finalized_block = *self.last_successful_block_no_snapshot_created.read();

		// // get all accounts
		let all_register_accounts = self
			.client
			.runtime_api()
			.get_all_accounts_and_proxies(&BlockId::number(last_finalized_block.saturated_into()))
			.map_err(|err| {
				JsonRpseeError::Custom((err.to_string() + "failed to get accounts").to_string())
			})?;

		// get snapshot summary
		let last_snapshot_summary = self
			.client
			.runtime_api()
			.get_latest_snapshot(&BlockId::number(last_finalized_block.saturated_into()))
			.map_err(|err| {
				JsonRpseeError::Custom(
					(err.to_string() + "failed to get snapshot summary").to_string(),
				)
			})?;

		// ToDo: Get all allow listed AssetIds
		// ToDo: Create existing DB
		// ToDo: Generate account info from existing DB
		// ToDo: st_id ?
		let ob_recovery_state = ObRecoveryState::new();
		let serialize_ob_recovery_state = serde_json::to_vec(&ob_recovery_state)?;
		Ok(serialize_ob_recovery_state)
	}
}
