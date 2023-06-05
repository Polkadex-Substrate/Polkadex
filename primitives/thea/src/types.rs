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

//! Definition of types used for `Thea` related operations.

#![cfg_attr(not(feature = "std"), no_std)]
use crate::{Network, ValidatorSetId};
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::Percent;

/// Defines the message structure.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Message {
	/// Block number.
	pub block_no: u64,
	/// Message nonce (e.g. identifier).
	pub nonce: u64,
	/// Payload of the message.
	pub data: Vec<u8>,
	/// Message originated from this network
	pub network: Network,
	/// Defines if authority was changed.
	pub is_key_change: bool,
	/// Validator set id at which this message was executed.
	pub validator_set_id: ValidatorSetId,
	/// Validators authorities set length.
	pub validator_set_len: u64,
}

impl Message {
	/// Calculates message validators threshold percentage.
	pub fn threshold(&self) -> u64 {
		const MAJORITY: u8 = 67;
		let p = Percent::from_percent(MAJORITY);
		p * self.validator_set_len
	}
}

#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

/// Defines structure of the deposit.
///
/// Deposit is relative to the "solochain".
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Deposit<AccountId> {
	/// Identifier of the deposit.
	pub id: Vec<u8>, // Unique identifier
	/// Receiver of the deposit.
	pub recipient: AccountId,
	/// Asset identifier.
	pub asset_id: u128,
	/// Amount of the deposit.
	pub amount: u128,
	/// Extra data.
	pub extra: Vec<u8>,
}

/// Defines the structure of the withdraw.
///
/// Withdraw is relative to solochain
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub struct Withdraw {
	/// Identifier of the withdrawal.
	pub id: Vec<u8>, // Unique identifier
	/// Asset identifier.
	pub asset_id: u128,
	/// Amount of the withdrawal.
	pub amount: u128,
	/// Receiver of the withdrawal.
	pub destination: Vec<u8>,
	/// Defines if withdraw operation is blocked.
	pub is_blocked: bool,
	/// Extra data.
	pub extra: Vec<u8>,
}
