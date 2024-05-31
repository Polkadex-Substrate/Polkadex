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

//! This module contains withdrawals related structures definition.

use crate::assets::AssetId;
use codec::{Decode, Encode, MaxEncodedLen};
use rust_decimal::Decimal;
use scale_info::TypeInfo;
use sp_core::H160;

use crate::{AccountId, Balance};
use serde::{Deserialize, Serialize};
use xcm::latest::Location;

#[derive(
	Encode, Decode, Clone, MaxEncodedLen, Debug, PartialEq, Eq, TypeInfo, Serialize, Deserialize,
)]
pub enum WithdrawalDestination {
	Polkadot(Location, Option<(AssetId, Balance)>),
}

/// Defines withdrawal structure.
#[derive(
	Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Serialize, Deserialize,
)]
pub struct Withdrawal<AccountId> {
	/// Withdrawal ID
	pub id: H160,
	/// Main account identifier.
	pub main_account: AccountId,
	/// Amount of withdrawal.
	pub amount: Decimal,
	/// Asset identifier.
	pub asset: AssetId,
	/// Fees of the withdraw operation.
	pub fees: Decimal,
	/// State change identifier.
	pub stid: u64,
	/// Cross-chain withdrawal destination
	pub destination: Option<WithdrawalDestination>,
}

/// Defines payload item structure collected in `Withdrawals` structure.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct WithdrawalPayload {
	/// Asset identifier.
	pub asset_id: AssetId,
	/// Amount of withdrawal.
	pub amount: Decimal,
	/// User's account identifier.
	pub user: AccountId,
}

/// Withdrawals collection wrapper structure definition.
#[derive(Encode, Decode, Debug, Clone, TypeInfo, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Withdrawals {
	/// Collection of withdrawals payloads.
	pub withdrawals: sp_std::vec::Vec<WithdrawalPayload>,
	/// Nonce (identifier).
	pub nonce: u32,
}
