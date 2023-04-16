//! Orderbook gadget specific errors
//!
//! Used for Orderbook gadget interal error handling only

use sp_api::ApiError;
use std::fmt::Debug;
use thea_primitives::Network;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Backend: {0}")]
	Backend(String),
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Scale codec error")]
	CodecError(parity_scale_codec::Error),
	#[error("Failed to submit incoming message to runtime")]
	FailedToSubmitMessageToRuntime,
	#[error("Signature verification Failed")]
	SignatureVerificationFailed,
	#[error("Network not configured for this validator, please use the rpc")]
	NetworkNotConfigured,
	#[error("BLS Signing failed")]
	SigningFailed,
	#[error("Block Hash not found")]
	BlockHashNotFound,
	#[error("Error while reading Thea Message")]
	ErrorReadingTheaMessage,
	#[error("Error from subxt: {0}")]
	Subxt(subxt::Error),
	#[error("Validator Set not initialized for netowrk: {0}")]
	ValidatorSetNotInitialized(Network),
}

impl From<subxt::Error> for Error {
	fn from(value: subxt::Error) -> Self {
		Self::Subxt(value)
	}
}

impl From<parity_scale_codec::Error> for Error {
	fn from(value: parity_scale_codec::Error) -> Self {
		Self::CodecError(value)
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
	fn from(value: ()) -> Self {
		Self::FailedToSubmitMessageToRuntime
	}
}

impl From<sc_keystore::Error> for Error {
	fn from(value: sc_keystore::Error) -> Self {
		Self::Keystore(value.to_string())
	}
}
