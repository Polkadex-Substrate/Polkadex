use sp_std::collections::btree_map::BTreeMap;
use parity_scale_codec::{Decode, Encode};
use rust_decimal::Decimal;
use scale_info::TypeInfo;
use crate::types::TradingPair;

/// All metrics used for calculating the LMP score of a main account
#[derive(Decode, Encode, TypeInfo, Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderMetric {
    pub maker_volume: Decimal, // Trading volume generated where main acc is a maker
    pub fees_paid: Decimal, // defined in terms of quote asset
    pub q_score: Decimal, // Market making performance score
    pub uptime: u16 // Uptime of market maker
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
    total_liquidity_mining_rewards: Decimal,
    /// Total rewards given in this epoch for trading
    total_trading_rewards: Decimal,
    /// % of Rewards allocated to each market from the pool
    market_weightage: BTreeMap<TradingPair, Decimal>,
    /// Min fees that should be paid to be eligible for rewards
    min_fees_paid: BTreeMap<TradingPair, Decimal>,
    /// Min maker volume for a marker to be eligible for rewards
    min_maker_volume: BTreeMap<TradingPair, Decimal>,
    /// Max number of accounts rewarded
    max_accounts_rewarded: u16,
    /// Claim safety period
    claim_safety_period: u32
}
