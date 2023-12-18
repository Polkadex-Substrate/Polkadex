use std::collections::BTreeMap;
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
#[derive(Decode, Encode, TypeInfo, Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct LMPOneMinuteReport<AccountId> {
    pub market: TradingPair,
    pub epoch: u16,
    pub index: u16, // Sample index out of 40,320 samples.
    // Sum of individual scores
    pub total_score: Decimal,
    // Final Scores of all eligible main accounts
    pub scores: BTreeMap<AccountId, Decimal>,
}