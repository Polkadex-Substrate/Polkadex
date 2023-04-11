use crate::{Network, ValidatorSetId};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq)]
pub struct Message {
	pub block_no: u64,
	pub nonce: u64,
	pub data: Vec<u8>,
	pub network: Network, // going out to this network
	pub is_key_change: bool,
	// ValidatorSetId at which this message was executed.
	pub validator_set_id: ValidatorSetId,
}

use crate::crypto::AuthorityId;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

pub fn set_bit_field(input: &mut Vec<u128>, bit_index: u16) {
	let required_elements = (bit_index % 128).saturating_add(1);
	if input.len() < required_elements as usize {
		input.resize(required_elements as usize, 0);
	}
	let element: usize = required_elements.saturating_sub(1) as usize;
	if element != 0 {
		input[element] |= 1 << (bit_index % (128 * element as u16));
	} else {
		input[element] |= 1 << (bit_index % 128);
	}
}

pub fn return_set_bits(input: &[u128]) -> Vec<u16> {
	let mut set_bits: Vec<u16> = Vec::new();

	for (element_index, element) in input.iter().enumerate() {
		for bit_index in 0..128u16 {
			if (element & (1 << bit_index)) == (1 << bit_index) {
				set_bits.push(bit_index.saturating_add((element_index * 128) as u16));
			}
		}
	}

	set_bits
}

pub fn prepare_bitmap(active: &Vec<AuthorityId>, local: &AuthorityId) -> Vec<u128> {
	let mut bitmap = Vec::new();
	if let Ok(index) = active.binary_search(local) {
		// Should be okay, validator cannot be more than 2000
		set_bit_field(&mut bitmap, index as u16);
	}

	bitmap
}
