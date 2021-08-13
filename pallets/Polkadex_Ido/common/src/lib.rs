#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
pub use polkadex_primitives::{AccountId, Balance, BlockNumber, Hash};
use polkadex_primitives::assets::AssetId;
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct FundingRoundWithPrimitives {
    pub token_a: AssetId,
    pub amount: Balance,
    pub token_b: AssetId,
    pub vesting_per_block: Balance,
    pub start_block: BlockNumber,
    pub min_allocation: Balance,
    pub max_allocation: Balance,
    pub operator_commission: Balance,
    pub token_a_priceper_token_b: Balance,
    pub close_round_block: BlockNumber,
    pub actual_raise: Balance,
}