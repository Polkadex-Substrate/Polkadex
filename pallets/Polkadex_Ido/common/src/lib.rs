#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use serde::{Serialize, Deserialize};
pub use polkadex_primitives::{AccountId, Balance, BlockNumber, Hash};
use polkadex_primitives::assets::AssetId;
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct FundingRoundWithPrimitives {
    pub token_a: AssetId,
    pub amount: Vec<u8>,
    pub token_b: AssetId,
    pub vesting_per_block: Vec<u8>,
    pub start_block: BlockNumber,
    pub min_allocation: Vec<u8>,
    pub max_allocation: Vec<u8>,
    pub operator_commission: Vec<u8>,
    pub token_a_priceper_token_b: Vec<u8>,
    pub close_round_block: BlockNumber,
    pub actual_raise: Vec<u8>,
}