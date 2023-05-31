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

use serde::{Deserialize, Serialize};
use serde_with::{json::JsonString, serde_as};
use std::collections::HashMap;

/// This is a dummy struct used to serialize memory db
/// We cannot serialize the hashmap below because of non-string type in key.
#[serde_as]
#[derive(Serialize, Deserialize)]
pub struct SnapshotStore {
	#[serde_as(as = "JsonString<Vec<(JsonString, _)>>")]
	pub map: HashMap<[u8; 32], (Vec<u8>, i32)>,
}
