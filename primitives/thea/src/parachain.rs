#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use xcm::{
	latest::{MultiLocation},
	VersionedMultiLocation,
};

/// Deposit is relative to solochain
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Deposit {
	pub recipient: MultiLocation,
	pub asset_id: u128,
	pub amount: u128,
}

/// Withdraw is relative to solochain
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Withdraw {
	pub asset_id: u128,
	pub amount: u128,
	pub destination: VersionedMultiLocation,
	pub is_blocked: bool
}
