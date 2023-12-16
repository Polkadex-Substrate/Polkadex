use parity_scale_codec::{Decode,Encode};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

/// All metrics used for calculating the LMP score of a main account
#[derive(Decode, Encode, TypeInfo, Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TraderMetric {
    pub maker_volume: Decimal, // Trading volume generated where main acc is a maker
    pub fees_paid: Decimal, // defined in terms of quote asset
    pub q_score: Decimal, // Market making performance score
    pub uptime: u16 // Uptime of market maker
}