#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::str;
use sp_std::vec::Vec;

use crate::data_structure::OrderType;
use sp_arithmetic::FixedU128;

#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum ErrorRpc {
    IdMustBe32Byte,
    Fixedu128tou128conversionFailed,
    AssetIdConversionFailed,
    NoElementFound,
    ServerErrorWhileCallingAPI,
}

#[derive(Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderTypeRPC {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
}


#[derive(Encode, Decode, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct Order4RPC {
    pub id: [u8; 32],
    pub trading_pair: [u8; 32],
    pub trader: [u8; 32],
    pub price: Vec<u8>,
    pub quantity: Vec<u8>,
    pub order_type: OrderType,
}


#[derive(Encode, Decode, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct LinkedPriceLevelRpc {
    pub next: Vec<u8>,
    pub prev: Vec<u8>,
    pub orders: Vec<Order4RPC>,
}


#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OrderbookRpc {
    pub trading_pair: [u8; 32],
    pub base_asset_id: u32,
    pub quote_asset_id: u32,
    pub best_bid_price: Vec<u8>,
    pub best_ask_price: Vec<u8>,
}


#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct MarketDataRpc {
    pub low: Vec<u8>,
    pub high: Vec<u8>,
    pub volume: Vec<u8>,
    pub open: Vec<u8>,
    pub close: Vec<u8>,
}

#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct OrderbookUpdates{
    pub bids: Vec<FrontendPricelevel>,
    pub asks: Vec<FrontendPricelevel>
}

#[derive(Encode, Decode, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct FrontendPricelevel {
    pub price: FixedU128,
    pub quantity: FixedU128
}