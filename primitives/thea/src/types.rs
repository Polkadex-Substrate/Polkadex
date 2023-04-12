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

use crate::crypto::AuthorityId;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

pub fn set_bit_field(input: &mut Vec<u128>, bit_index: usize) {
	let required_elements = (bit_index % 128).saturating_add(1);
	if input.len() < required_elements as usize {
		input.resize(required_elements as usize, 0);
	}
	let element: usize = required_elements.saturating_sub(1) as usize;
	if element != 0 {
		input[element] |= 1 << (bit_index % (128 * element));
	} else {
		input[element] |= 1 << (bit_index % 128);
	}
}

pub fn return_set_bits(input: &[u128]) -> Vec<usize> {
	let mut set_bits: Vec<usize> = Vec::new();

	for (element_index, element) in input.iter().enumerate() {
		for bit_index in 0..128usize {
			if (element & (1 << bit_index)) == (1 << bit_index) {
				set_bits.push(bit_index.saturating_add(element_index * 128));
			}
		}
	}

	set_bits
}

pub fn prepare_bitmap(active: &Vec<AuthorityId>, local: &AuthorityId) -> Vec<u128> {
	let mut bitmap = Vec::new();
	if let Ok(index) = active.binary_search(local) {
		// Should be okay, validator cannot be more than 2000
		set_bit_field(&mut bitmap, index);
	}

	bitmap
}
