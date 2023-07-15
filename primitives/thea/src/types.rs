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

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_std::cmp::Ordering;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

use crate::{Network, ValidatorSetId};

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

#[derive(Copy,
	Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize,
)]
pub enum Destination {
	Solochain,
	Parachain,
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

#[derive(Deserialize, Serialize, Clone)]
pub struct ApprovedMessage {
	pub message: Message,
	pub index: u16,
	pub signature: Vec<u8>,
	pub destination: Destination,
}

#[cfg(test)]
mod tests {
	use crate::{
		types::{ApprovedMessage, AssetMetadata, Destination},
		Message,
	};
	use parity_scale_codec::Encode;
	use sp_application_crypto::RuntimePublic;
	use polkadex_primitives::UNIT_BALANCE;
	use sp_core::{ByteArray, Pair};


	#[test]
	pub fn test_message_decode_encode() {
		let encoded_signature = "e7315de93b4ade67faa08195f43d54d9c76dbca2374968f13ae0a908a66624d746e46940262ef89f5afaece6652f4b2390652807ca3d67047c1e6fc15b28cbd901";

		let bytes = hex::decode(encoded_signature).unwrap();
		let pubk_bytes = hex::decode("0020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").unwrap();

		let signature = sp_core::ecdsa::Signature::from_slice(&bytes).unwrap();
		let pubk = sp_core::ecdsa::Public::from_slice(&bytes).unwrap();

		let msg = Message { block_no: 8,
			nonce: 1, data: [18, 52, 80],
			network: 1, is_key_change: false,
			validator_set_id: 0 };

		let msg_hash = sp_io::hashing::sha2_256(&msg.encode());

		pubk.verify(&msg_hash,&signature)

	}

	#[test]
	pub fn approved_message() {
		let message = Message {
			block_no: 1,
			nonce: 3,
			data: vec![1, 2, 3],
			network: 1,
			is_key_change: false,
			validator_set_id: 1,
		};
		let pair = sp_core::ecdsa::Pair::generate().0;
		let approved_message = ApprovedMessage {
			message: message.clone(),
			index: 0,
			signature: pair.sign(&message.encode()).encode(),
			destination: Destination::Solochain,
		};

		println!("{:?}", serde_json::to_string(&approved_message).unwrap())
	}

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
