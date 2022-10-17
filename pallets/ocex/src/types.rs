// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

use parity_scale_codec::u{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

/// Types that are required only in this pallet

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo, Clone, Copy)]
// #[scale_info(skip_type_params(WithdrawalLimit,SnapshotLimit))]
pub struct TradingPairStatus {
	/// Shows if this trading pair is active
	pub is_active: bool,
}

impl TradingPairStatus {
	pub fn new() -> Self {
		Self { is_active: true }
	}
}
