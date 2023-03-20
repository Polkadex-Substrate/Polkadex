#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

pub fn set_bit_field(input: &mut Vec<u128>, bit_index: u16) {
	let required_elements = (bit_index % 128).saturating_add(1);
	if input.len() < required_elements as usize {
		input.resize(required_elements as usize, 0);
	}
	let element: usize = required_elements.saturating_sub(1) as usize;
	if element != 0 {
		input[element] = input[element] | (1 << (bit_index % (128 * element as u16)));
	} else {
		input[element] = input[element] | (1 << (bit_index % 128));
	}
}

pub fn return_set_bits(input: &Vec<u128>) -> Vec<u16> {
	let mut set_bits: Vec<u16> = Vec::new();

	for element_index in 0..input.len() {
		let element = input[element_index];
		for bit_index in 0..128u16 {
			if (element & (1 << bit_index)) == (1 << bit_index) {
				set_bits.push(bit_index.saturating_add((element_index * 128) as u16));
			}
		}
	}

	set_bits
}

pub fn prepare_bitmap(indexes: Vec<u16>) -> Vec<u128> {
	let mut bitmap = Vec::new();
	for index in indexes {
		set_bit_field(&mut bitmap, index);
	}
	bitmap
}
