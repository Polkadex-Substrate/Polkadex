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

use sp_std::collections::btree_map::BTreeMap;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_system::Account;
use rust_decimal::Decimal;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};

/// A structure that represents the rewards information associated with an account.
#[derive(
	Clone,
	Encode,
	Decode,
	MaxEncodedLen,
	TypeInfo,
	Debug,
	PartialEq,
	Default,
	Serialize,
	Deserialize,
)]
pub struct RewardsInfoByAccount<Balance: Default> {
	/// The total amount of rewards that have been claimed by the account.
	pub claimed: Balance,

	/// The total amount of rewards that are unclaimed by the account but have
	/// been earned by participating in crowd loan
	/// provision).
	pub unclaimed: Balance,

	/// The total amount of rewards that are claimable by the account, meaning
	/// the rewards are currently available for the account to claim.
	pub claimable: Balance,
}

#[derive(
	Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Serialize, Deserialize,
)]
pub enum ExchangePayloadAction {
	Initialize,
	Claim,
}

#[derive(
	Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Serialize, Deserialize,
)]
pub struct ExchangePayload<AccountId> {
	pub reward_id: u32,
	pub action: ExchangePayloadAction,
	pub user: AccountId,
}

#[derive(
	Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq
)]
pub struct RewardProposal<AccountId> {
	pub user_reward: BTreeMap<AccountId, Reward>,
	pub is_passed: bool
}

impl<AccountId> RewardProposal<AccountId> {
	pub fn new(reward_map: BTreeMap<AccountId, Reward>) -> Self {
		Self {
			user_reward: reward_map,
			is_passed: false,
		}
	}

	pub fn approve_proposal(&mut self) {
		self.is_passed = true;
	}
}

#[derive(
	Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Serialize, Deserialize,
)]
pub struct Reward {
	pub amount: Decimal,
	pub is_claimed: bool
}
