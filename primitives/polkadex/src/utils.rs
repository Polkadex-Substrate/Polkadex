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

//! This module contains common/reusable utilities functions which performs low level operations and
//! could be reused in a different components.

#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

fn bit_expression_value(bit_index: usize) -> u128 {
	1 << (127 - (bit_index % 128))
}

pub fn set_bit_field(input: &mut [u128], bit_index: usize) -> bool {
	let element_pos = bit_index.div_floor(128);
	if element_pos >= input.len() {
		return false
	}
	input[element_pos] |= bit_expression_value(bit_index);
	true
}

/// Resolves indexes based on provided bitmap.
///
/// # Parameters
///
/// * `input`: Bitmap.
pub fn return_set_bits(input: &[u128]) -> Vec<usize> {
	let mut set_bits: Vec<usize> = Vec::new();
	for (element_index, element) in input.iter().enumerate() {
		for bit_index in 0..128usize {
			if (element & bit_expression_value(bit_index)) == bit_expression_value(bit_index) {
				set_bits.push(bit_index.saturating_add(element_index * 128));
			}
		}
	}

	set_bits
}

/// Calculates a bitmap based on provided indexes.
#[cfg(feature = "std")]
pub fn prepare_bitmap(indexes: &Vec<usize>, max_indexes: usize) -> Option<Vec<u128>> {
	// Sanity check
	for index in indexes {
		if *index > max_indexes {
			return None
		}
	}

	let total = max_indexes.div_floor(128).saturating_add(1);
	let mut bitmap = vec![0u128; total];
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

	#[test]
	pub fn test_bitmap_sample() {
		let input = vec![42535295865117307932921825928971026432];
		let set_bits: Vec<usize> = return_set_bits(&input);
		println!("Set bits: {:?}", set_bits);
	}
}
