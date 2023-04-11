//! Orderbook gadget specific errors
//!
//! Used for Orderbook gadget interal error handling only

use sp_api::ApiError;
use std::fmt::Debug;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
	#[error("Backend: {0}")]
	Backend(String),
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Scale codec error")]
	CodecError(parity_scale_codec::Error),
	#[error("Failed to submit snapshot to runtime")]
	FailedToSubmitSnapshotToRuntime,
	#[error("Signature verification Failed")]
	SignatureVerificationFailed,
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
