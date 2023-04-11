//! RPC API for Orderbook.

#![warn(missing_docs)]

use sc_rpc::SubscriptionTaskExecutor;

use futures::{channel::mpsc::UnboundedSender, task::SpawnError, SinkExt};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::{error::CallError, ErrorObject},
};
use thea_primitives::Network;

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
	/// Submits the validators network preference
	#[method(name = "thea_submitNetworkPref")]
	async fn submit_network_pref(&self, network: Network) -> RpcResult<()>;
}

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc {
	tx: UnboundedSender<Network>,
	_executor: SubscriptionTaskExecutor,
}

impl OrderbookRpc {
	/// Creates a new Orderbook Rpc handler instance.
	pub fn new(_executor: SubscriptionTaskExecutor, tx: UnboundedSender<Network>) -> Self {
		Self { tx, _executor }
	}
}

#[async_trait]
impl OrderbookApiServer for OrderbookRpc {
	async fn submit_network_pref(&self, network: Network) -> RpcResult<()> {
		let mut tx = self.tx.clone();
		tx.send(network).await?;
		Ok(())
	}
}
