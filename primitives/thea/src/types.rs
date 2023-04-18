#![cfg_attr(not(feature = "std"), no_std)]
use crate::{Network, ValidatorSetId};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::traits::Scale;

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Message {
	pub block_no: u64,
	pub nonce: u64,
	pub data: Vec<u8>,
	pub network: Network, // going out to this network
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

#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;
