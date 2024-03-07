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

use crate::types::TradingPair;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use rust_decimal::{
	prelude::{One, Zero},
	Decimal,
};
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::vec::Vec;

/// LMP Epoch config
#[derive(Decode, Encode, TypeInfo, Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LMPConfig {
	pub epoch: u16,
	pub index: u16,
}

/// One minute LMP Q Score report
#[derive(Decode, Encode, TypeInfo, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LMPOneMinuteReport<AccountId: Ord> {
	pub market: TradingPair,
	pub epoch: u16,
	pub index: u16, // Sample index out of 40,320 samples.
	// Sum of individual scores
	pub total_score: Decimal,
	// Final Scores of all eligible main accounts
	pub scores: BTreeMap<AccountId, Decimal>,
}

#[derive(Clone, Debug, Encode, Decode, Eq, PartialEq, TypeInfo, Serialize, Deserialize)]
pub struct LMPMarketConfigWrapper {
	pub trading_pair: TradingPair,
	pub market_weightage: u128,
	pub min_fees_paid: u128,
	pub min_maker_volume: u128,
	pub max_spread: u128,
	pub min_depth: u128,
}

/// LMP Configuration for a market
#[derive(
	Decode,
	Encode,
	TypeInfo,
	Clone,
	Copy,
	Debug,
	Eq,
	PartialEq,
	MaxEncodedLen,
	PartialOrd,
	Ord,
	Serialize,
	Deserialize,
)]
pub struct LMPMarketConfig {
	// % of Rewards allocated to each market from the pool
	pub weightage: Decimal,
	// Min fees that should be paid to be eligible for rewards
	pub min_fees_paid: Decimal,
	// Min maker volume for a marker to be eligible for rewards
	pub min_maker_volume: Decimal,
	// Max spread from mid-market price an Order can have to be eligible for LMP
	// We use quoted spread here, so the formula is
	// spread ( in % )  = ((midpoint - order price)/midpoint)*100
	// midpoint = average of best bid and ask price.

	// refer: https://en.wikipedia.org/wiki/Bid–ask_spread
	pub max_spread: Decimal,
	// Minimum depth an Order must have to be eligible for LMP
	// In Quote asset. ( it is basically volume of that order )
	pub min_depth: Decimal,
}

/// LMP Configuration for an epoch
#[serde_as]
#[derive(
	Decode, Encode, TypeInfo, Clone, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize,
)]
pub struct LMPEpochConfig {
	/// Total rewards given in this epoch for market making
	pub total_liquidity_mining_rewards: Decimal,
	/// Total rewards given in this epoch for trading
	pub total_trading_rewards: Decimal,
	/// Market Configurations
	#[serde_as(as = "Vec<(_, _)>")]
	pub config: BTreeMap<TradingPair, LMPMarketConfig>,
	/// Max number of accounts rewarded
	pub max_accounts_rewarded: u16,
	/// Claim safety period
	pub claim_safety_period: u32,
}

impl Default for LMPEpochConfig {
	fn default() -> Self {
		Self {
			total_liquidity_mining_rewards: Default::default(),
			total_trading_rewards: Default::default(),
			config: Default::default(),
			max_accounts_rewarded: 20,
			claim_safety_period: 50400,
		}
	}
}

impl LMPEpochConfig {
	/// Checks the integrity of current config
	pub fn verify(&self) -> bool {
		// Check if market weightage adds upto 1.0
		let mut total_percent = Decimal::zero();

		for config in self.config.values() {
			total_percent = total_percent.saturating_add(config.weightage);
		}

		if total_percent != Decimal::one() {
			return false;
		}

		true
	}
}
