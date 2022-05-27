// Copyright (C) 2020-2022 Polkadex OU
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

use crate::ValidatorSetId;
use codec::{Decode, Encode};
use sp_inherents::{InherentIdentifier, IsFatalError};

/// Thea Inherents
pub const INHERENT_IDENTIFIER: InherentIdentifier = *b"theapubk";

#[derive(Encode, Decode, sp_runtime::RuntimeDebug)]
pub struct TheaPublicKeyInherentDataType {
	pub public_key: Option<sp_core::ecdsa::Public>,
	pub set_id: ValidatorSetId,
}

/// Errors that can occur while checking the Thea inherent.
#[derive(Encode, sp_runtime::RuntimeDebug)]
#[cfg_attr(feature = "std", derive(Decode, thiserror::Error))]
pub enum InherentError {
	/// This is a fatal-error and will stop block import.
	#[cfg_attr(feature = "std", error("The inserted shared public key is invalid."))]
	InvalidPublicKey(TheaPublicKeyInherentDataType),
	/// This is a fatal-error and will stop block import.
	#[cfg_attr(feature = "std", error("Wrong Inherent Call in Block"))]
	WrongInherentCall,
}

impl IsFatalError for InherentError {
	fn is_fatal_error(&self) -> bool {
		match self {
			InherentError::InvalidPublicKey(_) => true,
			InherentError::WrongInherentCall => true,
		}
	}
}

impl InherentError {
	/// Try to create an instance ouf of the given identifier and data.
	#[cfg(feature = "std")]
	pub fn try_from(id: &InherentIdentifier, data: &[u8]) -> Option<Self> {
		if id == &INHERENT_IDENTIFIER {
			<InherentError as codec::Decode>::decode(&mut &data[..]).ok()
		} else {
			None
		}
	}
}
