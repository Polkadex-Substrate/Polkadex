//! Orderbook gadget specific errors
//!
//! Used for Orderbook gadget interal error handling only

use hash_db::MaybeDebug;
use orderbook_primitives::types::AccountAsset;
use std::fmt::Debug;
use trie_db::TrieError;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
	#[error("Backend: {0}")]
	Backend(String),
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Signature error: {0}")]
	Signature(String),
	#[error("Session uninitialized")]
	UninitSession,
	#[error("State hash mismatch")]
	StateHashMisMatch,
	#[error("OrderStateCheckFailed")]
	OrderStateCheckFailed,
	#[error("AccountBalanceNotFound in the state")]
	AccountBalanceNotFound(AccountAsset),
	#[error("Not enough balance in account")]
	InsufficientBalance,
	#[error("Error in trie computation")]
	TrieError(String),
	#[error("Scale codec error")]
	CodecError(parity_scale_codec::Error),
}

impl<T: MaybeDebug, E: MaybeDebug> From<Box<TrieError<T, E>>> for Error {
	fn from(value: Box<TrieError<T, E>>) -> Self {
		Self::TrieError(format!("{:?}", value))
	}
}

impl From<parity_scale_codec::Error> for Error {
	fn from(value: parity_scale_codec::Error) -> Self {
		Self::CodecError(value)
	}
}
