// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
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

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::ecdsa::Signature;

#[derive(
	Encode,
	Decode,
	Copy,
	Clone,
	Eq,
	PartialEq,
	Debug,
	TypeInfo,
	Serialize,
	Deserialize,
	MaxEncodedLen,
)]
pub enum Network {
	/// ETHEREUM Mainnet
	ETHEREUM,
	/// Not Supported
	NONE,
}

impl Default for Network {
	fn default() -> Self {
		Self::NONE
	}
}

/// Contains all the details for signing
///
/// Thea assumes that payload doesn't need to be processed again
#[derive(
	Encode,
	Decode,
	Debug,
	Clone,
	PartialEq,
	TypeInfo,
	Serialize,
	Deserialize,
	Default,
	MaxEncodedLen,
)]
pub struct UnsignedTheaPayload {
	/// Network Type
	pub network: Network,
	/// Payload for signing
	pub payload: [u8; 32],
	/// Payload submitted on block
	pub submission_blk: u32,
}

/// Contains both payload and valid signature
#[derive(Encode, Decode, Debug, Clone, PartialEq, TypeInfo, Default, MaxEncodedLen)]
pub struct SignedTheaPayload {
	/// Unsigned Payload
	pub payload: UnsignedTheaPayload,
	/// Valid Signature
	pub signature: Signature,
}
