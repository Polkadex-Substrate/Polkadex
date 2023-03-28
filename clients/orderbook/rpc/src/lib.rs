//! RPC API for Orderbook.

#![warn(missing_docs)]

use sc_rpc::SubscriptionTaskExecutor;

use futures::{channel::mpsc::UnboundedSender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use log::warn;
use orderbook_primitives::types::{ObMessage,ObRecoveryState};

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
	async fn get_orderbook_recovery_state(&self) -> RpcResult<ObRecoveryState>;
}

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc {
	tx: UnboundedSender<ObMessage>,
	_executor: SubscriptionTaskExecutor,
}

impl OrderbookRpc {
	/// Creates a new Orderbook Rpc handler instance.
	pub fn new(_executor: SubscriptionTaskExecutor, tx: UnboundedSender<ObMessage>) -> Self {
		Self { tx, _executor }
	}
}

#[async_trait]
impl OrderbookApiServer for OrderbookRpc {
	async fn submit_action(&self, message: ObMessage) -> RpcResult<()> {
		let mut tx = self.tx.clone();
		tx.send(message).await?;
		Ok(())
	}

	async fn get_orderbook_recovery_state(&self) -> RpcResult<ObRecoveryState> {
		let ob_recovery_state = ObRecoveryState::new();
		Ok(ob_recovery_state)
	}
}
