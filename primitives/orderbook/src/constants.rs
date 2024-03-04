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

//! This module contains constants definitions related to the "Orderbook".

use frame_support::PalletId;
use sp_core::crypto::{Ss58AddressFormat, Ss58Codec};
use sp_runtime::traits::AccountIdConversion;
use polkadex_primitives::{AccountId, Balance};

/// The designated SS58 prefix of this chain.
pub const POLKADEX_MAINNET_SS58: u16 = 88;

pub const MAX_WITHDRAWALS_PER_SNAPSHOT: u8 = 20;
pub const UNIT_BALANCE: Balance = 1_000_000_000_000_u128;
/// Range of QTY: 0.00000001 to 10,000,000 UNITs
pub const MIN_QTY: Balance = UNIT_BALANCE / 10000000;
pub const MAX_QTY: Balance = 10000000 * UNIT_BALANCE;
/// Range of PRICE: 0.00000001 to 10,000,000 UNITs
pub const MIN_PRICE: Balance = UNIT_BALANCE / 10000000;
pub const MAX_PRICE: Balance = 10000000 * UNIT_BALANCE;

pub const FEE_POT_PALLET_ID: PalletId = PalletId(*b"ocexfees");

#[test]
pub fn test_overflow_check() {
	assert!(MAX_PRICE.checked_mul(MAX_QTY).is_some());
}


#[test]
pub fn test_fee_pot_address(){
	let pot: AccountId = FEE_POT_PALLET_ID.into_account_truncating();
	println!("{:?}", pot.to_ss58check_with_version(Ss58AddressFormat::from(POLKADEX_MAINNET_SS58)))
}