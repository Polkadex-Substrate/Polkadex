use std::collections::HashMap;
use std::ops::Mul;
use std::borrow::Borrow;
use parity_scale_codec::{Codec, Decode, Encode};
use polkadex_primitives::{AccountId, AssetId, ocex::TradingPairConfig, Signature};
use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal::prelude::Zero;
use sp_core::H256;
use sp_std::cmp::Ordering;
use sp_std::collections::btree_map::BTreeMap;

use crate::SnapshotSummary;

pub type OrderId = H256;

/// Concrete implementation of Hasher using Blake2b 256-bit hashes
#[derive(Clone, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Snapshot {
    map: HashMap<H256, (Vec<u8>, i32)>,
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderState {
    pub filled_qty: Decimal,
    pub required_qty: Decimal,
}


impl OrderState {
    pub fn from(order: &Order) -> Self {
        // TODO: compute correct filled qty
        Self {
            filled_qty: order.filled_quantity,
            required_qty: order.qty,
        }
    }

    // verify if we can update the order state, with the new state of order.
    pub fn update(&mut self, order: &Order, price: Decimal, amount: Decimal) -> bool {
        todo!()
    }
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountInfo {
    proxies: Vec<AccountId>,
}

#[derive(Clone, Debug, Encode, Decode, Ord, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountAsset {
    main: AccountId,
    asset: AssetId,
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Trade {
    pub maker: Order,
    pub taker: Order,
    pub price: Decimal,
    pub amount: Decimal,
}

impl Trade {
    pub fn credit(&self, maker: bool) -> (AccountAsset, Decimal) {
        let user = if maker {
            self.maker.borrow()
        } else {
            self.taker.borrow()
        };
        let (base, quote) = (user.pair.base, user.pair.quote);
        match user.side {
            OrderSide::Ask => (AccountAsset{main: user.main_account.clone(), asset: quote}, self.price.mul(&self.amount)),
            OrderSide::Bid => (AccountAsset{main: user.main_account.clone(), asset: base}, self.amount)
        }
    }

    pub fn debit(&self, maker: bool) -> (AccountAsset, Decimal) {
        let user = if maker {
            self.maker.borrow()
        } else {
            self.taker.borrow()
        };
        let (base, quote) = (user.pair.base, user.pair.quote);
        match user.side {
            OrderSide::Ask => (AccountAsset{main: user.main_account.clone(), asset: base}, self.amount),
            OrderSide::Bid => (AccountAsset{main: user.main_account.clone(), asset: quote}, self.price.mul(&self.amount))
        }
    }
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ObMessage {
    pub stid: u64,
    pub action: UserActions,
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum GossipMessage {
    ObMessage(ObMessage),
    Snapshot(SnapshotSummary),
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum UserActions {
    Trade(Trade),
    Withdraw(WithdrawalRequest),
    BlockImport(u32),
}

#[derive(Clone, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalRequest {
    pub signature: Signature,
    pub payload: WithdrawPayloadCallByUser,
    pub main: AccountId,
    pub proxy: AccountId,
}

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawPayloadCallByUser {
    pub asset_id: AssetId,
    pub amount: String,
    pub timestamp: i64,
}

#[derive(Encode, Decode, Copy, Clone, Hash, Ord, PartialOrd, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderSide {
    Ask,
    Bid,
}

impl OrderSide {
    pub fn get_opposite(&self) -> Self {
        match self {
            OrderSide::Ask => OrderSide::Bid,
            OrderSide::Bid => OrderSide::Ask,
        }
    }
}

impl TryFrom<String> for OrderSide {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "Bid" => Ok(OrderSide::Bid),
            "Ask" => Ok(OrderSide::Ask),
            _ => Err(anyhow::Error::msg(format!("Unknown side variant: {:?}", value))),
        }
    }
}

#[derive(Encode, Decode, Copy, Clone, Hash, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderType {
    LIMIT,
    MARKET,
}

impl TryFrom<String> for OrderType {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "LIMIT" => Ok(OrderType::LIMIT),
            "MARKET" => Ok(OrderType::MARKET),
            _ => Err(anyhow::Error::msg("Unknown ot variant")),
        }
    }
}

#[derive(Encode, Decode, Copy, Clone, Hash, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderStatus {
    OPEN,
    CLOSED,
    CANCELLED,
}

impl TryFrom<String> for OrderStatus {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "OPEN" => Ok(OrderStatus::OPEN),
            "CLOSED" => Ok(OrderStatus::CLOSED),
            "CANCELLED" => Ok(OrderStatus::CANCELLED),
            _ => Err(anyhow::Error::msg("Unknown order status variant")),
        }
    }
}

impl Into<String> for OrderStatus {
    fn into(self) -> String {
        match self {
            Self::OPEN => "OPEN".to_string(),
            Self::CLOSED => "CLOSED".to_string(),
            Self::CANCELLED => "CANCELLED".to_string(),
        }
    }
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, Copy)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TradingPair {
    base: AssetId,
    quote: AssetId,
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Order {
    pub event_id: H256,
    pub client_order_id: H256,
    pub avg_filled_price: Decimal,
    pub fee: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    pub id: OrderId,
    pub user: AccountId,
    pub main_account: AccountId,
    pub pair: TradingPair,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub qty: Decimal,
    pub price: Decimal,
    pub quote_order_qty: Decimal,
    pub timestamp: i64,
    pub overall_unreserved_volume: Decimal,
}

impl Order {
    pub fn verify_config(&self, _config: &TradingPairConfig) -> bool {
        todo!()
    }
}

impl PartialOrd for Order {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.side != other.side {
            return None;
        }
        if self.side == OrderSide::Bid {
            // Buy side
            match self.price.cmp(&other.price) {
                Ordering::Less => Some(Ordering::Greater),
                Ordering::Equal =>
                    if self.timestamp < other.timestamp {
                        Some(Ordering::Less)
                    } else {
                        Some(Ordering::Greater)
                    },
                Ordering::Greater => Some(Ordering::Less),
            }
        } else {
            // Sell side
            match self.price.cmp(&other.price) {
                Ordering::Less => Some(Ordering::Less),
                Ordering::Equal => {
                    // If price is equal, we follow the FIFO priority
                    if self.timestamp < other.timestamp {
                        Some(Ordering::Less)
                    } else {
                        Some(Ordering::Greater)
                    }
                }
                Ordering::Greater => Some(Ordering::Greater),
            }
        }
    }
}

impl Ord for Order {
    fn cmp(&self, other: &Self) -> Ordering {
        assert_eq!(self.side, other.side, "Comparison cannot work for opposite order sides");
        if self.side == OrderSide::Bid {
            // Buy side
            match self.price.cmp(&other.price) {
                Ordering::Less => Ordering::Less,
                Ordering::Equal =>
                    if self.timestamp < other.timestamp {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    },
                Ordering::Greater => Ordering::Greater,
            }
        } else {
            // Sell side
            match self.price.cmp(&other.price) {
                Ordering::Less => Ordering::Greater,
                Ordering::Equal => {
                    // If price is equal, we follow the FIFO priority
                    if self.timestamp < other.timestamp {
                        Ordering::Greater
                    } else {
                        Ordering::Less
                    }
                }
                Ordering::Greater => Ordering::Less,
            }
        }
    }
}

impl Order {
    /// Computes the new avg_price and adds qty to filled_qty
    /// if returned is false then underflow occurred during division
    pub fn update_avg_price_and_filled_qty(&mut self, price: Decimal, amount: Decimal) -> bool {
        let mut temp = self.avg_filled_price.saturating_mul(self.filled_quantity);
        temp = temp.saturating_add(amount.saturating_mul(price));
        self.filled_quantity = self.filled_quantity.saturating_add(amount);
        println!("self.filled_quantity: {:?}\ntemp: {:?}", self.filled_quantity, temp);
        match temp.checked_div(self.filled_quantity) {
            Some(quotient) => {
                println!("Quotient: {:?}", quotient);
                self.avg_filled_price = quotient;
                true
            }
            None => false,
        }
    }

    pub fn available_volume(&self, other_price: Option<Decimal>) -> Decimal {
        //this if for market bid order
        if self.qty.is_zero() {
            println!(
                "quote_order_qty: {:?}, avg_filled_price: {:?}, filled_quantity: {:?}",
                self.quote_order_qty, self.avg_filled_price, self.filled_quantity
            );
            return rounding_off(
                self.quote_order_qty
                    .saturating_sub(self.avg_filled_price.saturating_mul(self.filled_quantity)),
            );
        }
        //this is for market ask order
        if self.order_type == OrderType::MARKET {
            rounding_off(
                self.qty
                    .saturating_sub(self.filled_quantity)
                    .saturating_mul(other_price.unwrap_or_default()),
            )
        }
        //this is for limit orders
        else {
            // We cannot use avg. price here as limit orders might not have avg_price defined
            // if they are not yet matched and just inserted into the book
            rounding_off(self.qty.saturating_sub(self.filled_quantity).saturating_mul(self.price))
        }
    }
}

pub fn rounding_off(a: Decimal) -> Decimal {
    a.round_dp_with_strategy(8, RoundingStrategy::ToZero)
}

#[cfg(test)]
mod tests {
    use crate::types::{ObMessage, UserActions};

    #[test]
    pub fn test_ob_message() {
        let msg = ObMessage { stid: 0, action: UserActions::BlockImport(1) };

        println!("OBMessage: {:?}", serde_json::to_string(&msg).unwrap());
    }
}
