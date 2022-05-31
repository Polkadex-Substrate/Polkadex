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

#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use polkadex_primitives::assets::AssetId;
pub use polkadex_primitives::{AccountId, Balance, BlockNumber, Hash};
#[cfg(feature = "std")]
use serde::{ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer};
use sp_runtime::SaturatedConversion;
use sp_std::vec::Vec;
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct FundingRoundWithPrimitives<AccountId> {
	pub token_a: StringAssetId,
	pub creator: AccountId,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub amount: Balance,
	pub token_b: StringAssetId,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub vesting_per_block: Balance,
	pub start_block: BlockNumber,
	pub vote_end_block: BlockNumber,
	pub vesting_end_block: BlockNumber,
	pub project_info_cid: Vec<u8>,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub min_allocation: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub max_allocation: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub token_a_priceper_token_b: Balance,
	pub close_round_block: BlockNumber,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub actual_raise: Balance,
}

#[cfg(feature = "std")]
fn serialize_as_string<S: Serializer, T: std::fmt::Display>(
	t: &T,
	serializer: S,
) -> Result<S::Ok, S::Error> {
	serializer.serialize_str(&t.to_string())
}

#[cfg(feature = "std")]
fn deserialize_from_string<'de, D: Deserializer<'de>, T: std::str::FromStr>(
	deserializer: D,
) -> Result<T, D::Error> {
	let s = String::deserialize(deserializer)?;
	s.parse::<T>().map_err(|_| serde::de::Error::custom("Parse from string failed"))
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub struct VoteStat {
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub yes: Balance,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	pub no: Balance,
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", serde(rename_all = "camelCase"))]
pub enum StringAssetId {
	POLKADEX,
	#[cfg_attr(feature = "std", serde(serialize_with = "serialize_as_string"))]
	#[cfg_attr(feature = "std", serde(deserialize_with = "deserialize_from_string"))]
	Asset(u128),
}

impl From<AssetId> for StringAssetId {
	fn from(asset: AssetId) -> Self {
		match asset {
			AssetId::POLKADEX => StringAssetId::POLKADEX,
			AssetId::Asset(num) => StringAssetId::Asset(num.into()),
		}
	}
}
