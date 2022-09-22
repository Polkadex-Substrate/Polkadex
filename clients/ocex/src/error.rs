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

//! OCEX gadget specific errors
//!
//! Used for internal error handling only

use log::error;
use sp_api::ApiError;
use ocex_primitives::{ValidatorSetId};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
	#[error("Keystore error: {0}")]
	Keystore(String),
	#[error("Signature error: {0}")]
	Signature(String),
	#[error("UnableToFindAuthorityFromKeystore")]
	UnableToFindAuthorityFromKeystore,
	#[error("Error calling runtime api: {0}")]
	RuntimeApiError(String),
	#[error("Thea Runtime Api Error: {0}")]
	TheaRuntimeApiError(String),
	#[error("Error: {0}")]
	Other(String),
}

impl From<ApiError> for Error {
	fn from(err: ApiError) -> Self {
		Self::RuntimeApiError(err.to_string())
	}
}