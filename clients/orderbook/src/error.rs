// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

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
	#[error("Block trying to be imported is not finalized yet: finalized: {0}, importing: {1}")]
	BlockNotFinalized(u64, u64),
	#[error("Snapshot Data not found in DB")]
	SnapshotNotFound,
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
	#[error("Failed to submit snapshot to runtime")]
	FailedToSubmitSnapshotToRuntime,
	#[error("Offchain storage not available")]
	OffchainStorageNotAvailable,
	#[error("Signature verification Failed")]
	SignatureVerificationFailed,
	#[error("Invalid trade found")]
	InvalidTrade,
	#[error("Unable to find trading pair config")]
	TradingPairConfigNotFound,
	#[error("Error during BLS operation: {0}")]
	BLSError(String),
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

impl From<blst::BLST_ERROR> for Error {
	fn from(value: blst::BLST_ERROR) -> Self {
		Self::BLSError(format!("{value:?}"))
	}
}
