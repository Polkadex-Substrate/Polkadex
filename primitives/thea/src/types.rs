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

//! Definition of types used for `Thea` related operations.

#![cfg_attr(not(feature = "std"), no_std)]

use std::collections::BTreeSet;
use binary_merkle_tree::merkle_root;
use sp_std::collections::btree_map::BTreeMap;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_std::cmp::Ordering;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

use crate::{Network, ValidatorSetId};

#[derive(
Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize,
)]
pub enum OnChainMessage{
	KR1(Vec<u8>),
	KR2(BTreeMap<[u8; 32], Vec<u8>>),
	VerifyngKey([u8;65]),
	SR1(Vec<u8>),
	SR2([u8;32]),
	SR3(([u8; 32], u8, [u8; 32], [u8; 32], [u8; 20]))
}

#[derive(
Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialOrd, Deserialize, Serialize,
)]
pub struct AggregatedPayload {
	/// Validator set id at which this message was executed.
	pub validator_set_id: ValidatorSetId,
	/// Messages (is_key_change, Data)
	pub messages: BTreeSet<MessageV2>,
	/// Defines if authority was changed.
	pub is_key_change: bool,
}

impl Ord for AggregatedPayload {
	fn cmp(&self, other: &Self) -> Ordering {
		match (self.is_key_change, other.is_key_change) {
			(true, false) => Ordering::Less,
			(false,true) => Ordering::Greater,
			(true, true) => Ordering::Equal,
			(false, false) => Ordering::Equal
		}
	}
}

impl AggregatedPayload {
	/// Returns the merkle root of all messages
	pub fn root(&self) -> H256 {
		let messages: Vec<[u8;32]> = self.messages.iter().map(|x| sp_io::hashing::keccak_256(&x.encode())).collect();
		merkle_root::<sp_core::KeccakHasher, _>(messages)
	}
}


/// Defines the message structure in thea version2
#[derive(
Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize,
)]
pub struct MessageV2 {
	/// Message nonce (e.g. identifier).
	pub nonce: u64,
	/// Payload of the message.
	pub data: Vec<u8>,
	/// Message originated from this network if it's an incoming message
	/// and destination network if it's an outgoing message
	pub network: Network,
}

impl From<Message> for MessageV2 {
	fn from(value: Message) -> Self {
		MessageV2{
			nonce: value.nonce,
			data: value.data,
			network: value.network,
		}
	}
}

/// Defines the message structure.
#[derive(
	Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd, Deserialize, Serialize,
)]
pub struct Message {
	/// Block number.
	pub block_no: u64,
	/// Message nonce (e.g. identifier).
	pub nonce: u64,
	/// Payload of the message.
	pub data: Vec<u8>,
	/// Message originated from this network if it's an incoming message
	/// and destination network if it's an outgoing message
	pub network: Network,
	/// Defines if authority was changed.
	pub is_key_change: bool,
	/// Validator set id at which this message was executed.
	pub validator_set_id: ValidatorSetId,
}

/// Defines the destination of a thea message
#[derive(
	Copy,
	Clone,
	Encode,
	Decode,
	TypeInfo,
	Debug,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Serialize,
	Deserialize,
)]
pub enum Destination {
	Solochain,
	Parachain,
	Aggregator,
}

/// Defines structure of the deposit.
///
/// Deposit is relative to the "solochain".
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Deposit<AccountId> {
	/// Identifier of the deposit.
	pub id: Vec<u8>, // Unique identifier
	/// Receiver of the deposit.
	pub recipient: AccountId,
	/// Asset identifier.
	pub asset_id: u128,
	/// Amount of the deposit.
	pub amount: u128,
	/// Extra data.
	pub extra: Vec<u8>,
}

impl<AccountId> Deposit<AccountId> {
	pub fn amount_in_native_decimals(&self, metadata: AssetMetadata) -> u128 {
		metadata.convert_to_native_decimals(self.amount)
	}

	pub fn amount_in_foreign_decimals(&self, metadata: AssetMetadata) -> u128 {
		metadata.convert_from_native_decimals(self.amount)
	}
}

/// Defines the structure of the withdraw.
///
/// Withdraw is relative to solochain
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Withdraw {
	/// Identifier of the withdrawal.
	pub id: Vec<u8>,
	// Unique identifier
	/// Asset identifier.
	pub asset_id: u128,
	/// Amount of the withdrawal.
	pub amount: u128,
	/// Receiver of the withdrawal.
	pub destination: Vec<u8>,
	/// Defines if withdraw operation is blocked.
	pub is_blocked: bool,
	/// Extra data.
	pub extra: Vec<u8>,
}

/// Metadata of asset's decimals
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug, Copy)]
pub struct AssetMetadata {
	decimal: u8,
}

impl AssetMetadata {
	pub fn new(decimal: u8) -> Option<AssetMetadata> {
		if decimal < 1 {
			return None
		}
		Some(AssetMetadata { decimal })
	}

	/// Convert the foreign asset amount to native decimal configuration
	pub fn convert_to_native_decimals(&self, amount: u128) -> u128 {
		let diff = 12 - self.decimal as i8;
		match diff.cmp(&0) {
			Ordering::Less => {
				// casting should not fail as diff*-1 is positive
				amount.saturating_div(10u128.pow((-diff) as u32))
			},
			Ordering::Equal => amount,
			Ordering::Greater => amount.saturating_mul(10u128.pow(diff as u32)),
		}
	}

	/// Convert the foreign asset amount from native decimal configuration
	pub fn convert_from_native_decimals(&self, amount: u128) -> u128 {
		let diff = 12 - self.decimal as i8;

		match diff.cmp(&0) {
			Ordering::Less => {
				// casting should not fail as diff*-1 is positive
				amount.saturating_mul(10u128.pow((-diff) as u32))
			},
			Ordering::Equal => amount,
			Ordering::Greater => amount.saturating_div(10u128.pow(diff as u32)),
		}
	}
}

/// Overarching type used by aggregator to collect signatures from
/// authorities for a given Thea message
#[derive(Deserialize, Serialize, Clone)]
pub struct ApprovedMessage {
	/// Thea message
	pub message: Message,
	/// index of the authority from on-chain list
	pub index: u16,
	/// ECDSA signature of authority
	pub signature: Vec<u8>,
	/// Destination network
	pub destination: Destination,
}

#[cfg(test)]
mod tests {
	use crate::types::AssetMetadata;
	use polkadex_primitives::UNIT_BALANCE;

	#[test]
	pub fn test_decimal_conversion() {
		// Decimal is greater
		let greater = AssetMetadata::new(18).unwrap();
		assert_eq!(
			greater.convert_to_native_decimals(1_000_000_000_000_000_000_u128),
			UNIT_BALANCE
		);
		assert_eq!(
			greater.convert_from_native_decimals(UNIT_BALANCE),
			1_000_000_000_000_000_000_u128
		);
		assert_eq!(
			greater.convert_to_native_decimals(1_234_567_891_234_567_890_u128),
			1_234_567_891_234_u128
		);
		assert_eq!(
			greater.convert_from_native_decimals(1_234_567_891_234_u128),
			1_234_567_891_234_000_000_u128
		);

		// Decimal is same
		let same = AssetMetadata::new(12).unwrap();
		assert_eq!(same.convert_to_native_decimals(UNIT_BALANCE), UNIT_BALANCE);
		assert_eq!(same.convert_from_native_decimals(UNIT_BALANCE), UNIT_BALANCE);

		// Decimal is lesser
		let smaller = AssetMetadata::new(8).unwrap();
		assert_eq!(smaller.convert_to_native_decimals(100_000_000), UNIT_BALANCE);
		assert_eq!(smaller.convert_from_native_decimals(UNIT_BALANCE), 100_000_000);
		assert_eq!(smaller.convert_to_native_decimals(12_345_678u128), 123_456_780_000u128);
		assert_eq!(smaller.convert_from_native_decimals(123_456_789_123u128), 12_345_678u128);
	}
}
