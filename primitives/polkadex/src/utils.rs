#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

fn bit_expression_value(bit_index: u16) -> u128 {
	1 << (127 - (bit_index % 128))
}

pub fn set_bit_field(input: &mut [u128], bit_index: u16) -> bool {
	let element_pos = bit_index.div_floor(128) as usize;
	if element_pos >= input.len() {
		return false
	}
	input[element_pos] |= bit_expression_value(bit_index);
	true
}

pub fn return_set_bits(input: &[u128]) -> Vec<u16> {
	let mut set_bits: Vec<u16> = Vec::new();
	for (element_index, element) in input.iter().enumerate() {
		for bit_index in 0..128u16 {
			if (element & bit_expression_value(bit_index)) == bit_expression_value(bit_index) {
				set_bits.push(bit_index.saturating_add((element_index * 128) as u16));
			}
		}
	}

	set_bits
}

#[cfg(feature = "std")]
pub fn prepare_bitmap(indexes: &Vec<u16>, max_indexes: u16) -> Option<Vec<u128>> {
	// Sanity check
	for index in indexes {
		if *index > max_indexes {
			return None
		}
	}

	let total = max_indexes.div_floor(128).saturating_add(1);
	let mut bitmap = vec![0u128; total as usize];
	for index in indexes {
		if !set_bit_field(&mut bitmap, *index) {
			return None
		}
	}
	Some(bitmap)
}

#[cfg(test)]
mod tests {
	use crate::utils::{prepare_bitmap, return_set_bits};

	#[test]
	pub fn test_prepare_bitmap() {
		let input = vec![1, 3, 5];
		let map = prepare_bitmap(&input, 5).unwrap();
		assert_eq!(map, vec![111655151645932933323919793063548944384u128]);
		assert_eq!(return_set_bits(&map), input);
	}
}
