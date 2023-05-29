//! Orderbook gadget specific errors
//!
//! Used for Orderbook gadget interal error handling only

use std::fmt::Debug;

use sp_api::ApiError;
use tokio::task::JoinError;

use thea_primitives::Network;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Backend: {0}")]
	Backend(String),
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Scale codec error")]
	CodecError(String),
	#[error("Failed to submit incoming message to runtime")]
	FailedToSubmitMessageToRuntime,
	#[error("Network not configured for this validator, please use the rpc")]
	NetworkNotConfigured,
	#[error("Error while reading Thea Message")]
	ErrorReadingTheaMessage,
	#[error("Error from subxt: {0}")]
	Subxt(String),
	#[error("Validator Set not initialized for netowrk: {0}")]
	ValidatorSetNotInitialized(Network),
	#[error("Error during BLS operation: {0}")]
	BLSError(String),
	#[error("No validators found on runtime")]
	NoValidatorsFound,
}

impl From<subxt::Error> for Error {
	fn from(value: subxt::Error) -> Self {
		Self::Subxt(value.to_string())
	}
}

impl From<parity_scale_codec::Error> for Error {
	fn from(value: parity_scale_codec::Error) -> Self {
		Self::CodecError(value.to_string())
	}
}

impl From<ApiError> for Error {
	fn from(value: ApiError) -> Self {
		Self::Backend(value.to_string())
	}
}

impl From<JoinError> for Error {
	fn from(value: JoinError) -> Self {
		Self::Backend(value.to_string())
	}
}

impl From<()> for Error {
	fn from(_: ()) -> Self {
		Self::FailedToSubmitMessageToRuntime
	}
}

impl From<sc_keystore::Error> for Error {
	fn from(value: sc_keystore::Error) -> Self {
		Self::Keystore(value.to_string())
	}
}

impl From<bls_primitives::Error> for Error {
	fn from(value: bls_primitives::Error) -> Self {
		Self::BLSError(format!("{value:?}"))
	}
}
