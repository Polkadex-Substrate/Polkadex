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

use crate::{AuthorityIndex, ValidatorSetId};
use codec::{Codec, Decode, Encode, MaxEncodedLen};
use frame_support::{storage::bounded_btree_set::BoundedBTreeSet, traits::Get, BoundedVec};
use scale_info::TypeInfo;

#[derive(Encode, Decode, PartialEq, Debug, TypeInfo, Clone, MaxEncodedLen)]
pub enum SubProtocol {
	Keygen,
	OfflineStage,
}

pub trait ProvideSubProtocol {
	fn subprotocol() -> SubProtocol;
}

/// Struct containing MPC messages
#[derive(Encode, Decode, TypeInfo, Clone, PartialEq, Debug, MaxEncodedLen)]
pub struct Msg<MsgLimit: Get<u32>> {
	pub receiver: Option<u16>,
	pub message: BoundedVec<u8, MsgLimit>,
	pub sender: u16,
}

/// Thea Signing Round
#[derive(Encode, Decode, TypeInfo, Clone, Copy, PartialEq, Debug, MaxEncodedLen)]
pub enum SigningRound {
	Round0,
}

impl Default for SigningRound {
	fn default() -> Self {
		Self::Round0
	}
}

/// Different stages of Keygen sub protocol
#[derive(Encode, Decode, TypeInfo, Clone, Copy, PartialEq, Debug, MaxEncodedLen)]
pub enum KeygenRound {
	Round0,
	// Won't be used in Msg
	Round1,
	Round2,
	Round3,
	Round4,
	Round5,
	// Won't be used in Msg
	Unknown,
}

impl Default for KeygenRound {
	fn default() -> Self {
		Self::Unknown
	}
}

impl Into<u8> for KeygenRound {
	fn into(self) -> u8 {
		match self {
			KeygenRound::Round0 => 0,
			KeygenRound::Round1 => 1,
			KeygenRound::Round2 => 2,
			KeygenRound::Round3 => 3,
			KeygenRound::Round4 => 4,
			KeygenRound::Round5 => 5,
			KeygenRound::Unknown => u8::MAX,
		}
	}
}

impl ProvideSubProtocol for KeygenRound {
	fn subprotocol() -> SubProtocol {
		SubProtocol::Keygen
	}
}

/// Different stages of OfflineStage sub protocol
#[derive(Encode, Decode, TypeInfo, Clone, Copy, PartialEq, Debug, MaxEncodedLen)]
pub enum OfflineStageRound {
	Round0,
	// Won't be used in Msg
	Round1,
	Round2,
	Round3,
	Round4,
	Round5,
	Round6,
	Round7,
	// Won't be used in Msg
	Unknown,
}

impl Into<u8> for OfflineStageRound {
	fn into(self) -> u8 {
		match self {
			OfflineStageRound::Round0 => 0,
			OfflineStageRound::Round1 => 1,
			OfflineStageRound::Round2 => 2,
			OfflineStageRound::Round3 => 3,
			OfflineStageRound::Round4 => 4,
			OfflineStageRound::Round5 => 5,
			OfflineStageRound::Round6 => 6,
			OfflineStageRound::Round7 => 7,
			OfflineStageRound::Unknown => u8::MAX,
		}
	}
}

impl From<u8> for OfflineStageRound {
	fn from(data: u8) -> Self {
		match data {
			0 => Self::Round0,
			1 => Self::Round1,
			2 => Self::Round2,
			3 => Self::Round3,
			4 => Self::Round4,
			5 => Self::Round5,
			6 => Self::Round6,
			7 => Self::Round7,
			_ => Self::Unknown,
		}
	}
}

impl Default for OfflineStageRound {
	fn default() -> Self {
		Self::Unknown
	}
}

/// Keygen Payload for unsigned transaction with signed payload
#[derive(Encode, Decode, Clone, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
pub struct TheaPayload<
	AuthorityId,
	SubProtocolRound: Codec + Default,
	MsgLimit: Get<u32> + Clone,
	MsgVecLimit: Get<u32> + Clone,
> {
	pub messages: BoundedVec<Msg<MsgLimit>, MsgVecLimit>,
	pub signer: Option<AuthorityId>,
	pub set_id: ValidatorSetId,
	pub auth_idx: AuthorityIndex,
	pub round: SubProtocolRound,
}

impl<
		AuthorityId,
		SubProtocolRound: Codec + Default,
		MsgLimit: Get<u32> + Clone,
		MsgVecLimit: Get<u32> + Clone,
	> Default for TheaPayload<AuthorityId, SubProtocolRound, MsgLimit, MsgVecLimit>
{
	fn default() -> Self {
		Self {
			messages: BoundedVec::default(),
			signer: None,
			set_id: 0,
			auth_idx: 0,
			round: SubProtocolRound::default(),
		}
	}
}

pub type PartialSignature<PartialSignatureLimit> = BoundedVec<u8, PartialSignatureLimit>;

#[derive(Encode, Decode, Clone, PartialEq, Debug, TypeInfo, MaxEncodedLen)]
pub struct SigningSessionPayload<
	AuthorityId,
	PartialSignatureLimit: Get<u32> + Clone,
	PartialSignatureVecLimit: Get<u32> + Clone,
> {
	/// Here each element is serialized partial signature
	///
	/// Also, lenght of list should be equal to unsignedPayloads in that block
	pub partial_signatures:
		BoundedVec<PartialSignature<PartialSignatureLimit>, PartialSignatureVecLimit>,
	pub signer: Option<AuthorityId>,
	pub set_id: ValidatorSetId,
	pub auth_idx: AuthorityIndex,
}
impl<
		AuthorityId,
		PartialSignatureLimit: Get<u32> + Clone,
		PartialSignatureVecLimit: Get<u32> + Clone,
	> Default for SigningSessionPayload<AuthorityId, PartialSignatureLimit, PartialSignatureVecLimit>
{
	fn default() -> Self {
		Self { partial_signatures: BoundedVec::default(), signer: None, set_id: 0, auth_idx: 0 }
	}
}

impl From<u16> for KeygenRound {
	fn from(round: u16) -> Self {
		match round {
			0 => KeygenRound::Round0,
			1 => KeygenRound::Round1,
			2 => KeygenRound::Round2,
			3 => KeygenRound::Round3,
			4 => KeygenRound::Round4,
			5 => KeygenRound::Round5,
			_ => KeygenRound::Unknown,
		}
	}
}

impl From<u16> for OfflineStageRound {
	fn from(round: u16) -> Self {
		match round {
			0 => OfflineStageRound::Round0,
			1 => OfflineStageRound::Round1,
			2 => OfflineStageRound::Round2,
			3 => OfflineStageRound::Round3,
			4 => OfflineStageRound::Round4,
			5 => OfflineStageRound::Round5,
			6 => OfflineStageRound::Round6,
			7 => OfflineStageRound::Round7,
			_ => OfflineStageRound::Unknown,
		}
	}
}

impl ProvideSubProtocol for OfflineStageRound {
	fn subprotocol() -> SubProtocol {
		SubProtocol::OfflineStage
	}
}

/// An offense report to be submitted by validators
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo, MaxEncodedLen)]
pub struct OffenseReport<AuthorityId> {
	/// The offender
	pub offender: AuthorityId,
	/// The block at which the message had to be sent
	pub offense_blk: u32,
	/// The round at which the message had to be sent
	pub protocol: SubProtocol,
	/// The validator that authored this report 
	pub author: AuthorityId
}
