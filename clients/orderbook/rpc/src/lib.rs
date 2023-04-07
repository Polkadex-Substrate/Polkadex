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
use orderbook_primitives::types::ObMessage;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::party_i::SignatureRecid;
use curv::elliptic::curves::{secp256_k1::Secp256k1, Curve, Point, Scalar};
use curv::arithmetic::Converter;

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
	async fn submit_action(
		&self,
		action: ObMessage,
		signature_r: Scalar<Secp256k1>,
		signature_s: Scalar<Secp256k1>
	) -> RpcResult<()>;
}

/// Implements the OrderbookApi RPC trait for interacting with Orderbook.
pub struct OrderbookRpc {
	tx: UnboundedSender<ObMessage>,
	_executor: SubscriptionTaskExecutor,
}

impl OrderbookRpc {
	/// Creates a new Orderbook Rpc handler instance.
	pub fn new(
		_executor: SubscriptionTaskExecutor,
		tx: UnboundedSender<ObMessage>,
	) -> Self {
		Self { tx, _executor }
	}
}

#[async_trait]
impl OrderbookApiServer for OrderbookRpc {
	async fn submit_action(
		&self,
		message: ObMessage,
		signature_r: Scalar<Secp256k1>,
		signature_s: Scalar<Secp256k1>
	) -> RpcResult<()> {
// Since KMS does not return V value, It is assumed that it is either 27 or 28
		// So only if both function returns false, it is a signature verification error
		let mut tx = self.tx.clone();
		tx.send(message).await?;
		Ok(())
	}
}
