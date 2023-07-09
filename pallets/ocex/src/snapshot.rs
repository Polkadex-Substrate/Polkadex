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

use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::BlockNumber;

// Accounts storage
#[derive(Encode, Decode, PartialEq, Debug, Clone, Default)]
pub struct StateInfo {
	/// Last block processed
	pub last_block: BlockNumber,
	/// Last processed worker nonce
	pub worker_nonce: u64,
	/// Last processed stid
	pub stid: u64,
	/// Last processed snapshot id
	pub snapshot_id: u64,
}
