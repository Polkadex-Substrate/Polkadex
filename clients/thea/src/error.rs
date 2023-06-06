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

//! Thea gadget specific errors.
//!
//! Used for Thea gadget internal error handling only.

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
