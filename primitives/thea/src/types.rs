#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::UNIT_BALANCE;
use scale_info::TypeInfo;
use sp_runtime::{traits::Scale, Saturating};
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

use crate::{Network, ValidatorSetId};

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Message {
	pub block_no: u64,
	pub nonce: u64,
	pub data: Vec<u8>,
	// Message originated from this network
	pub network: Network,
	pub is_key_change: bool,
	// ValidatorSetId at which this message was executed.
	pub validator_set_id: ValidatorSetId,
	pub validator_set_len: u64,
}

impl Message {
	pub fn threshold(&self) -> u64 {
		self.validator_set_len.saturating_mul(2).div(3u64)
	}
}

/// Deposit is relative to solochain
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Deposit<AccountId> {
	pub id: Vec<u8>, // Unique identifier
	pub recipient: AccountId,
	pub asset_id: u128,
	pub amount: u128,
	pub extra: Vec<u8>,
}

/// Withdraw is relative to solochain
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Withdraw {
	pub id: Vec<u8>,
	// Unique identifier
	pub asset_id: u128,
	pub amount: u128,
	pub destination: Vec<u8>,
	pub is_blocked: bool,
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
		if diff > 0 {
			amount.saturating_mul(10u128.pow(diff as u32))
		} else if diff == 0 {
			amount
		} else {
			// casting should not fail as diff*-1 is positive
			amount.saturating_div(10u128.pow((diff * -1) as u32))
		}
	}

	/// Convert the foreign asset amount from native decimal configuration
	pub fn convert_from_native_decimals(&self, amount: u128) -> u128 {
		let diff = 12 - self.decimal as i8;
		if diff > 0 {
			amount.saturating_div(10u128.pow(diff as u32))
		} else if diff == 0 {
			amount
		} else {
			// casting should not fail as diff*-1 is positive
			amount.saturating_mul(10u128.pow((diff * -1) as u32))
		}
	}
}

#[test]
pub fn test_decimal_conversion() {
	// Decimal is greater
	let greater = AssetMetadata::new(18).unwrap();
	assert_eq!(greater.convert_to_native_decimals(1000_000_000_000_000_000u128), UNIT_BALANCE);
	assert_eq!(greater.convert_from_native_decimals(UNIT_BALANCE), 1000_000_000_000_000_000u128);
	assert_eq!(
		greater.convert_to_native_decimals(1234_567_891_234_567_890u128),
		1234_567_891_234u128
	);
	assert_eq!(
		greater.convert_from_native_decimals(1234_567_891_234u128),
		1234_567_891_234_000_000u128
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
