use crate::types::TradingPair;
use parity_scale_codec::{Decode, Encode};
use rust_decimal::{
	prelude::{One, Zero},
	Decimal,
};
use scale_info::TypeInfo;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::vec::Vec;

/// All metrics used for calculating the LMP score of a main account
#[derive(Decode, Encode, TypeInfo, Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderMetric {
	pub maker_volume: Decimal, // Trading volume generated where main acc is a maker
	pub fees_paid: Decimal,    // defined in terms of quote asset
	pub q_score: Decimal,      // Market making performance score
	pub uptime: u16,           // Uptime of market maker
}

/// One minute LMP Q Score report
#[derive(Decode, Encode, TypeInfo, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct LMPOneMinuteReport<AccountId: Ord> {
	pub market: TradingPair,
	pub epoch: u16,
	pub index: u16, // Sample index out of 40,320 samples.
	// Sum of individual scores
	pub total_score: Decimal,
	// Final Scores of all eligible main accounts
	pub scores: BTreeMap<AccountId, Decimal>,
}

/// LMP Configuration for an epoch
#[derive(Decode, Encode, TypeInfo, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct LMPEpochConfig {
	/// Total rewards given in this epoch for market making
	pub total_liquidity_mining_rewards: Decimal,
	/// Total rewards given in this epoch for trading
	pub total_trading_rewards: Decimal,
	/// % of Rewards allocated to each market from the pool
	pub market_weightage: BTreeMap<TradingPair, Decimal>,
	/// Min fees that should be paid to be eligible for rewards
	pub min_fees_paid: BTreeMap<TradingPair, Decimal>,
	/// Min maker volume for a marker to be eligible for rewards
	pub min_maker_volume: BTreeMap<TradingPair, Decimal>,
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
			market_weightage: Default::default(),
			min_fees_paid: Default::default(),
			min_maker_volume: Default::default(),
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
		for percent in self.market_weightage.values() {
			total_percent = total_percent.saturating_add(*percent);
		}
		if total_percent != Decimal::one() {
			return false
		}

		// Make sure all three maps' keys are identical
		let keys1: Vec<_> = self.market_weightage.keys().collect();
		let keys2: Vec<_> = self.min_fees_paid.keys().collect();
		let keys3: Vec<_> = self.min_maker_volume.keys().collect();

		if keys1 != keys2 || keys2 != keys3 {
			return false
		}
		true
	}
}
