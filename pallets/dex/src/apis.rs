use codec::{Decode, Encode};
use sp_arithmetic::FixedU128;
use sp_std::vec::Vec;



#[derive(Encode, Decode, PartialEq)]
pub struct PriceLevelData {
    pub price_level: FixedU128,
    pub amount: FixedU128,
}

#[derive(Encode, Decode, PartialEq)]
pub struct OrderBookApi {
    pub bids: Vec<PriceLevelData>,
    pub asks: Vec<PriceLevelData>,
    pub enabled: bool,
}