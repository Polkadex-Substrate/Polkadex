//! Orderbook gadget specific errors
//!
//! Used for Orderbook gadget interal error handling only

use hash_db::MaybeDebug;
use orderbook_primitives::types::AccountAsset;
use sp_api::ApiError;
use std::fmt::Debug;
use tokio::task::JoinError;
use trie_db::TrieError;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
	#[error("Backend: {0}")]
	Backend(String),
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("State hash mismatch")]
	StateHashMisMatch,
	#[error("AccountBalanceNotFound in the state")]
	AccountBalanceNotFound(AccountAsset),
	#[error("Not enough balance in account")]
	InsufficientBalance,
	#[error("Error in trie computation")]
	TrieError(String),
	#[error("Scale codec error")]
	CodecError(parity_scale_codec::Error),
	#[error("Signature check failed for withdraw")]
	WithdrawSignatureCheckFailed,
	#[error("Decimal library error")]
	DecimalError(rust_decimal::Error),
	#[error("Unable to find main account in trie")]
	MainAccountNotFound,
	#[error("Unable to find proxy account in trie")]
	ProxyAccountNotFound,
	#[error("Proxy not associated with main")]
	ProxyNotAssociatedWithMain,
	// #[error("Error while snapshot signing")]
	// SnapshotSigningFailed,
	#[error("Failed to submit snapshot to runtime")]
	FailedToSubmitSnapshotToRuntime,
	#[error("Main account already registered")]
	MainAlreadyRegistered,
	#[error("Proxy account already registered")]
	ProxyAlreadyRegistered,
	#[error("Offchain storage not available")]
	OffchainStorageNotAvailable,
	#[error("Signature verification Failed")]
	SignatureVerificationFailed,
	#[error("Invalid trade found")]
	InvalidTrade,
	#[error("Unable to find trading pair config")]
	TradingPairConfigNotFound,
}

impl<T: MaybeDebug, E: MaybeDebug> From<Box<TrieError<T, E>>> for Error {
	fn from(value: Box<TrieError<T, E>>) -> Self {
		Self::TrieError(format!("{value:?}"))
	}
}

impl From<parity_scale_codec::Error> for Error {
	fn from(value: parity_scale_codec::Error) -> Self {
		Self::CodecError(value)
	}
}

impl From<rust_decimal::Error> for Error {
	fn from(value: rust_decimal::Error) -> Self {
		Self::DecimalError(value)
	}
}

impl From<ApiError> for Error {
	fn from(value: ApiError) -> Self {
		Self::Backend(value.to_string())
	}
}

impl From<reqwest::Error> for Error {
	fn from(value: reqwest::Error) -> Self {
		Self::Backend(value.to_string())
	}
}

impl From<JoinError> for Error {
	fn from(value: JoinError) -> Self {
		Self::Backend(value.to_string())
	}
}

impl From<sc_keystore::Error> for Error {
	fn from(value: sc_keystore::Error) -> Self {
		Self::Keystore(value.to_string())
	}
}
