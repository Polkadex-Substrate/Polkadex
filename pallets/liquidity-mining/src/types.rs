// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

#[derive(Encode, Decode, MaxEncodedLen, TypeInfo)]
pub struct MarketMakerConfig<AccountId> {
	pub pool_id: AccountId,
	pub commission: Decimal,
	pub exit_fee: Decimal,
	pub public_funds_allowed: bool,
	pub name: [u8; 10],
	pub share_id: u128,
	pub force_closed: bool,
}

pub type EpochNumber = u32;
