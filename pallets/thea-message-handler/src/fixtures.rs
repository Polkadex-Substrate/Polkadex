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

use crate::*;
use frame_support::BoundedVec;

pub(crate) const SIG: [u8; 48] = [
	149, 78, 11, 39, 209, 149, 209, 101, 74, 132, 154, 96, 46, 218, 114, 207, 95, 52, 40, 70, 44,
	13, 7, 236, 224, 87, 192, 58, 99, 125, 175, 25, 35, 186, 6, 53, 246, 152, 164, 191, 169, 212,
	133, 30, 143, 196, 55, 214,
];

pub(crate) const PK: [u8; 96] = [
	128, 68, 92, 111, 149, 140, 246, 244, 137, 50, 23, 217, 197, 153, 235, 255, 228, 58, 108, 191,
	41, 203, 237, 112, 203, 173, 118, 41, 92, 3, 165, 18, 200, 173, 125, 232, 182, 162, 9, 122, 13,
	77, 41, 222, 92, 53, 60, 0, 22, 227, 136, 163, 35, 121, 27, 34, 208, 233, 191, 74, 36, 223, 17,
	34, 79, 35, 164, 208, 138, 207, 171, 53, 254, 213, 17, 141, 35, 196, 81, 247, 20, 171, 33, 187,
	152, 79, 229, 3, 121, 17, 242, 252, 147, 209, 50, 186,
];

pub(crate) fn produce_authorities<T: Config>(
) -> BoundedVec<<T as crate::Config>::TheaId, <T as crate::Config>::MaxAuthorities> {
	let mut authorities: BoundedVec<
		<T as crate::Config>::TheaId,
		<T as crate::Config>::MaxAuthorities,
	> = BoundedVec::with_bounded_capacity(1);
	let key = <T as crate::Config>::TheaId::decode(&mut PK.as_ref()).unwrap();
	authorities.try_push(key).unwrap();
	authorities
}
