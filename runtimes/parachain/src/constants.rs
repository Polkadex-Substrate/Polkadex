// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

pub mod currency {
	pub type Balance = u128;
	pub const PDEX: Balance = 1_000_000_000_000;
	pub const DOLLARS: Balance = PDEX; // 1_000_000_000_000
	pub const CENTS: Balance = DOLLARS / 100; // 10_000_000_000
}
