
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

//! Constants for pallet-thea

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::traits::Get;
use scale_info::TypeInfo;

// TODO: Implement the types below using a macro

#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MsgLimit;
impl Get<u32> for MsgLimit {
	fn get() -> u32 {
		20000 // TODO got from test_encode_decode in thea client, probably wrong. needs fix
	}
}

#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MsgVecLimit;
impl Get<u32> for MsgVecLimit {
	fn get() -> u32 {
		600 // 100 validators * 6 rounds
	}
}
#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct PartialSignatureLimit;
impl Get<u32> for PartialSignatureLimit {
	fn get() -> u32 {
		600 // 100 validators * 6 rounds
	}
}
#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct PartialSignatureVecLimit;
impl Get<u32> for PartialSignatureVecLimit {
	fn get() -> u32 {
		600 // 100 validators * 6 rounds
	}
}

#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct OffenceReportBTreeSetLimit;
impl Get<u32> for OffenceReportBTreeSetLimit {
	fn get() -> u32 {
		100
	}
}
