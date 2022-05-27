// This file is part of Substrate.

// Copyright (C) 2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! THEA gadget specific errors
//!
//! Used for THEA gadget interal error handling only

use log::error;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::state_machine::{
	keygen::Error as StateMachineKeygenError,
	sign::{Error as StateMachineOfflineStageError, SignError},
};
use round_based::IsCritical;
use sp_api::ApiError;
use std::{fmt::Debug, num::ParseIntError};
use thea_primitives::{SigningError, ValidatorSetId};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Signature error: {0}")]
	Signature(String),
	#[error("UnableToFindAuthorityFromKeystore")]
	UnableToFindAuthorityFromKeystore,
	#[error("Round Not found error: {0}")]
	RoundNotFound(ValidatorSetId),
	#[error("Stage Not found")]
	StageNotFound,
	#[error("Local Party Not Initialized error")]
	LocalPartyNotInitialized,
	#[error("Offline Party Not Initialized error")]
	OfflinePartyNotInitialized,
	#[error("Attempted to Pick Output before protocol completion")]
	ProtocolNotComplete,
	#[error("Error in encoding/decoding: {0}")]
	SerdeError(String),
	#[error("No block in Queue with Unsigned Payload")]
	NoBlockInQueue,
	#[error("Critical StateMachine error")]
	CriticalKeygenStateMachineError,
	#[error("StateMachine error")]
	StateKeygenMachineError,
	#[error("Critical OfflineStage StateMachine error")]
	CriticalOfflineStageStateMachineError,
	#[error("OfflineStage StateMachine error")]
	StateOfflineStageMachineError,
	#[error("local key is not initialized yet")]
	LocalKeyNotReady,
	#[error("No Pending Payloads")]
	NoPayloadPending,
	#[error("Error calling runtime api: {0}")]
	RuntimeApiError(String),
	#[error("Thea Runtime Api Error: {0}")]
	TheaRuntimeApiError(String),
	#[error("Error during ECDSA Signature generation: {0}")]
	ECDSASignatureError(String),
	#[error("Unable to find signing session")]
	UnableToFindSigningSession,
	#[error("Integer overflow")]
	IntegerOverflow,
	#[error("Given vector/slice/btreeset overflows the bounded version's limit")]
	BoundedVecOrSliceError,
	#[error("Libsecp256k1 Error: {0}")]
	Libsecp256k1error(String),
	#[error("Error: {0}")]
	Other(String),
}

impl From<serde_json::Error> for Error {
	fn from(err: serde_json::Error) -> Self {
		Self::SerdeError(err.to_string())
	}
}

impl From<StateMachineKeygenError> for Error {
	fn from(err: StateMachineKeygenError) -> Self {
		if err.is_critical() {
			error!(target: "thea", "Critical State machine error: {:?}", err);
			Self::CriticalKeygenStateMachineError
		} else {
			error!(target: "thea", " State machine error: {:?}", err);
			Self::StateKeygenMachineError
		}
	}
}

impl From<()> for Error {
	fn from(_x: ()) -> Self {
		Self::BoundedVecOrSliceError
	}
}

impl From<StateMachineOfflineStageError> for Error {
	fn from(err: StateMachineOfflineStageError) -> Self {
		if err.is_critical() {
			error!(target: "thea", "Critical State machine error: {:?}", err);
			Self::CriticalOfflineStageStateMachineError
		} else {
			error!(target: "thea", " State machine error: {:?}", err);
			Self::StateOfflineStageMachineError
		}
	}
}

impl From<ApiError> for Error {
	fn from(err: ApiError) -> Self {
		Self::RuntimeApiError(err.to_string())
	}
}
impl From<SigningError> for Error {
	fn from(err: SigningError) -> Self {
		Self::TheaRuntimeApiError(err.to_string())
	}
}

impl From<SignError> for Error {
	fn from(err: SignError) -> Self {
		Self::ECDSASignatureError(err.to_string())
	}
}

impl From<ParseIntError> for Error {
	fn from(err: ParseIntError) -> Self {
		Self::Other(err.to_string())
	}
}

impl From<secp256k1::Error> for Error {
	fn from(err: secp256k1::Error) -> Self {
		Self::Libsecp256k1error(err.to_string())
	}
}
