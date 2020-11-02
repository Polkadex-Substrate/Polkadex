#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::ensure;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_arithmetic::traits::CheckedDiv;
use sp_std::collections::vec_deque::VecDeque;
use sp_std::convert::TryInto;
use sp_std::str;
use sp_std::vec::Vec;

use crate::data_structure_rpc::{ErrorRpc, LinkedPriceLevelRpc, MarketDataRpc, Order4RPC, OrderbookRpc};
use crate::Trait;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OrderType {
    BidLimit,
    BidMarket,
    AskLimit,
    AskMarket,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct Order<T> where T: Trait {
    pub id: T::Hash,
    pub trading_pair: T::Hash,
    pub trader: T::AccountId,
    pub price: FixedU128,
    pub quantity: FixedU128,
    pub order_type: OrderType,
}

impl<T> Order<T> where T: Trait {
    pub fn convert(self) -> Result<Order4RPC, ErrorRpc> {
        let order = Order4RPC {
            id: Self::account_to_bytes(&self.id)?,
            trading_pair: Self::account_to_bytes(&self.trading_pair)?,
            trader: Self::account_to_bytes(&self.trader)?,
            price: Self::convert_fixed_u128_to_balance(self.price).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            quantity: Self::convert_fixed_u128_to_balance(self.quantity).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            order_type: self.order_type,
        };
        Ok(order)
    }

    fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], ErrorRpc>
        where AccountId: Encode,
    {
        let account_vec = account.encode();
        ensure!(account_vec.len() == 32, ErrorRpc::IdMustBe32Byte);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&account_vec);
        Ok(bytes)
    }

    pub fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(balance_in_u128.encode())
        } else {
            None
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct LinkedPriceLevel<T> where T: Trait {
    pub next: Option<FixedU128>,
    pub prev: Option<FixedU128>,
    pub orders: VecDeque<Order<T>>,
}

impl<T> LinkedPriceLevel<T> where T: Trait {
    pub fn convert(self) -> Result<LinkedPriceLevelRpc, ErrorRpc> {
        let linked_pirce_level = LinkedPriceLevelRpc {
            next: Self::convert_fixed_u128_to_balance(self.next.ok_or(ErrorRpc::NoElementFound)?).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            prev: Self::convert_fixed_u128_to_balance(self.prev.ok_or(ErrorRpc::NoElementFound)?).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            orders: Self::cov_de_vec(self.clone().orders)?,
        };
        Ok(linked_pirce_level)
    }

    fn cov_de_vec(temp: VecDeque<Order<T>>) -> Result<Vec<Order4RPC>, ErrorRpc> {
        let mut temp3: Vec<Order4RPC> = Vec::new();
        for element in temp {
            temp3.push(element.convert()?)
        };
        Ok(temp3)
    }

    fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();

            let hex_vec: Vec<u8> = balance_in_u128.encode();
            Some(hex_vec)
        } else {
            None
        }
    }
}

impl<T> Default for LinkedPriceLevel<T> where T: Trait {
    fn default() -> Self {
        LinkedPriceLevel {
            next: None,
            prev: None,
            orders: Default::default(),
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct Orderbook<T> where T: Trait {
    pub trading_pair: T::Hash,
    pub base_asset_id: T::AssetId,
    pub quote_asset_id: T::AssetId,
    pub best_bid_price: FixedU128,
    pub best_ask_price: FixedU128,
}

impl<T> Orderbook<T> where T: Trait {
    pub fn convert(self) -> Result<OrderbookRpc, ErrorRpc> {
        let orderbook = OrderbookRpc {
            trading_pair: Self::account_to_bytes(&self.trading_pair)?,
            base_asset_id: TryInto::<u32>::try_into(self.base_asset_id).ok().ok_or(ErrorRpc::AssetIdConversionFailed)?,
            quote_asset_id: TryInto::<u32>::try_into(self.quote_asset_id).ok().ok_or(ErrorRpc::AssetIdConversionFailed)?,
            best_bid_price: Self::convert_fixed_u128_to_balance(self.best_bid_price).ok_or(ErrorRpc::IdMustBe32Byte)?,
            best_ask_price: Self::convert_fixed_u128_to_balance(self.best_ask_price).ok_or(ErrorRpc::IdMustBe32Byte)?,
        };
        Ok(orderbook)
    }

    fn account_to_bytes<AccountId>(account: &AccountId) -> Result<[u8; 32], ErrorRpc>
        where AccountId: Encode,
    {
        let account_vec = account.encode();
        ensure!(account_vec.len() == 32, ErrorRpc::IdMustBe32Byte);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&account_vec);
        Ok(bytes)
    }

    fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(balance_in_u128.encode())
        } else {
            None
        }
    }
}

impl<T> Default for Orderbook<T> where T: Trait {
    fn default() -> Self {
        Orderbook {
            trading_pair: T::Hash::default(),
            base_asset_id: 0.into(),
            quote_asset_id: 0.into(),
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0),
        }
    }
}

impl<T> Orderbook<T> where T: Trait {
    pub fn new(base_asset_id: T::AssetId, quote_asset_id: T::AssetId, trading_pair: T::Hash) -> Self {
        Orderbook {
            trading_pair,
            base_asset_id,
            quote_asset_id,
            best_bid_price: FixedU128::from(0),
            best_ask_price: FixedU128::from(0),
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct MarketData {
    // Lowest price at which the trade was executed in a block.
    pub low: FixedU128,
    // Highest price at which the trade was executed in a block.
    pub high: FixedU128,
    // Total volume traded in a block.
    pub volume: FixedU128,
    // Opening price for this block.
    pub open: FixedU128,
    // Closing price for this block.
    pub close: FixedU128,
}

impl MarketData {
    pub fn convert(self) -> Result<MarketDataRpc, ErrorRpc> {
        let market_data = MarketDataRpc {
            low: Self::convert_fixed_u128_to_balance(self.low).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            high: Self::convert_fixed_u128_to_balance(self.high).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            volume: Self::convert_fixed_u128_to_balance(self.volume).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            open: Self::convert_fixed_u128_to_balance(self.open).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
            close: Self::convert_fixed_u128_to_balance(self.close).ok_or(ErrorRpc::Fixedu128tou128conversionFailed)?,
        };
        Ok(market_data)
    }

    fn convert_fixed_u128_to_balance(x: FixedU128) -> Option<Vec<u8>> {
        if let Some(balance_in_fixed_u128) = x.checked_div(&FixedU128::from(1000000)) {
            let balance_in_u128 = balance_in_fixed_u128.into_inner();
            Some(balance_in_u128.encode())
        } else {
            None
        }
    }
}

impl Default for MarketData {
    fn default() -> Self {
        MarketData {
            low: FixedU128::from(0),
            high: FixedU128::from(0),
            volume: FixedU128::from(0),
            open: FixedU128::from(0),
            close: FixedU128::from(0),
        }
    }
}
