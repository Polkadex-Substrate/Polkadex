use crate::constants::*;
use parity_scale_codec::{Codec, Decode, Encode};
use polkadex_primitives::{BlockNumber,ocex::TradingPairConfig, withdrawal::Withdrawal, AccountId, AssetId, Signature};
use rust_decimal::{prelude::Zero, Decimal, RoundingStrategy};
use sp_core::H256;
use sp_runtime::traits::Verify;
use sp_std::{cmp::Ordering, collections::btree_map::BTreeMap};
use std::{
	borrow::Borrow,
	collections::HashMap,
	fmt::{Display, Formatter},
	ops::Mul,
	str::FromStr,
};

use crate::SnapshotSummary;

pub type OrderId = H256;


/// A struct representing the recovery state of an Order Book.
#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ObRecoveryState {
	/// The snapshot ID of the order book recovery state.
	pub snapshot_id: u64,
	/// A `BTreeMap` that maps main account to a vector of proxy account.
	pub account_ids: BTreeMap<AccountId, Vec<AccountId>>,
	/// A `BTreeMap` that maps `AccountAsset`s to `Decimal` balances.
	pub balances: BTreeMap<AccountAsset, Decimal>,
	/// The last block number that was processed by validator.
	pub last_process_block_number: BlockNumber,
}

impl ObRecoveryState {
	pub fn new() -> Self {
		ObRecoveryState{
			snapshot_id: 0,
			account_ids: BTreeMap::default(),
			balances: BTreeMap::default(),
			last_process_block_number: 0,
		}
	}

	pub fn add_balance(&self) {

	}

	pub fn add_account(&self) {

	}
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
		Self { filled_qty: order.filled_quantity, required_qty: order.qty }
	}

	// verify if we can update the order state, with the new state of order.
	pub fn update(&mut self, order: &Order, price: Decimal, amount: Decimal) -> bool {
		// Verify signature also here.
		// TODO: FIX this.
		true
	}
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountInfo {
	pub proxies: Vec<AccountId>,
}

#[derive(Clone, Debug, Encode, Decode, Ord, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountAsset {
	pub main: AccountId,
	pub asset: AssetId,
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Trade {
	pub maker: Order,
	pub taker: Order,
	pub price: Decimal,
	pub amount: Decimal,
	pub time: i64,
}

impl Trade {
	pub fn credit(&self, maker: bool) -> (AccountAsset, Decimal) {
		let user = if maker { self.maker.borrow() } else { self.taker.borrow() };
		let (base, quote) = (user.pair.base, user.pair.quote);
		match user.side {
			OrderSide::Ask => (
				AccountAsset { main: user.main_account.clone(), asset: quote },
				self.price.mul(&self.amount),
			),
			OrderSide::Bid =>
				(AccountAsset { main: user.main_account.clone(), asset: base }, self.amount),
		}
	}

	pub fn debit(&self, maker: bool) -> (AccountAsset, Decimal) {
		let user = if maker { self.maker.borrow() } else { self.taker.borrow() };
		let (base, quote) = (user.pair.base, user.pair.quote);
		match user.side {
			OrderSide::Ask =>
				(AccountAsset { main: user.main_account.clone(), asset: base }, self.amount),
			OrderSide::Bid => (
				AccountAsset { main: user.main_account.clone(), asset: quote },
				self.price.mul(&self.amount),
			),
		}
	}
}
#[cfg(feature = "std")]
use chrono::Utc;
use libp2p_core::PeerId;

impl Trade {
	// Creates a Trade with zero event_tag
	#[cfg(feature = "std")]
	pub fn new(maker: Order, taker: Order, price: Decimal, amount: Decimal) -> Trade {
		Self { maker, taker, price, amount, time: Utc::now().timestamp_millis() }
	}

	// Verifies the contents of a trade
	pub fn verify(&self) -> bool {
		// TODO: Verify the signatures of both orders
		//  Validity of both orders
		//
		todo!()
	}
}

#[cfg(feature = "std")]
#[derive(Clone, Debug, Encode, Decode, serde::Serialize, serde::Deserialize)]
pub enum GossipMessage {
	// (From,to, remote peer)
	WantStid(u64, u64),
	// Collection of Stids
	Stid(Vec<ObMessage>),
	// Single ObMessage
	ObMessage(ObMessage),
	// Snapshot id, bitmap, remote peer
	Want(u64, Vec<u128>),
	// Snapshot id, bitmap, remote peer
	Have(u64, Vec<u128>),
	// Request
	// (snapshot id, chunk indexes requested as bitmap, remote peer)
	RequestChunk(u64, Vec<u128>),
	// Chunks of snapshot data
	// ( snapshot id, index of chunk, data )
	Chunk(u64, u16, Vec<u8>),
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ObMessage {
	pub stid: u64,
	pub action: UserActions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StateSyncStatus {
	Unavailable, // We don't have this chunk yet
	// (Who is supposed to send us, when we requested)
	InProgress(PeerId, i64), // We have asked a peer for this chunk and waiting
	Available,               // We have this chunk
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum UserActions {
	Trade(Vec<Trade>),
	Withdraw(WithdrawalRequest),
	BlockImport(u32),
	Snapshot,
}

#[derive(Clone, Debug, Decode, Encode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalRequest {
	pub signature: Signature,
	pub payload: WithdrawPayloadCallByUser,
	pub main: AccountId,
	pub proxy: AccountId,
}

impl TryInto<Withdrawal<AccountId>> for WithdrawalRequest {
	type Error = rust_decimal::Error;

	fn try_into(self) -> Result<Withdrawal<AccountId>, rust_decimal::Error> {
		Ok(Withdrawal {
			main_account: self.main.clone(),
			amount: self.amount()?,
			asset: self.payload.asset_id,
			event_id: 0,              // TODO: We don't use this
			fees: Default::default(), // TODO: We don't use this
		})
	}
}

impl WithdrawalRequest {
	pub fn verify(&self) -> bool {
		self.signature.verify(self.payload.encode().as_ref(), &self.proxy)
	}

	pub fn account_asset(&self) -> AccountAsset {
		AccountAsset { main: self.main.clone(), asset: self.payload.asset_id }
	}

	pub fn amount(&self) -> Result<Decimal, rust_decimal::Error> {
		Decimal::from_str(&self.payload.amount)
	}
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

#[derive(Encode, Decode, Copy, Hash, Ord, PartialOrd, Clone, PartialEq, Debug, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TradingPair {
	pub base: AssetId,
	pub quote: AssetId,
}

#[cfg(feature = "std")]
impl TryFrom<String> for TradingPair {
	type Error = anyhow::Error;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		let assets: Vec<&str> = value.split('-').collect();
		if assets.len() != 2 {
			return Err(anyhow::Error::msg("Invalid String"))
		}

		let base_asset = if assets[0] == String::from("PDEX").as_str() {
			AssetId::polkadex
		} else {
			let id = assets[0].parse::<u128>()?;
			AssetId::asset(id)
		};

		let quote_asset = if assets[1] == String::from("PDEX").as_str() {
			AssetId::polkadex
		} else {
			let id = assets[1].parse::<u128>()?;
			AssetId::asset(id)
		};

		Ok(TradingPair::from(quote_asset, base_asset))
	}
}

impl TradingPair {
	pub fn from(quote: AssetId, base: AssetId) -> Self {
		TradingPair { base, quote }
	}

	pub fn is_quote_asset(&self, asset_id: AssetId) -> bool {
		self.quote == asset_id
	}
	pub fn is_base_asset(&self, asset_id: AssetId) -> bool {
		self.base == asset_id
	}

	pub fn is_part_of(&self, asset_id: AssetId) -> bool {
		(self.base == asset_id) | (self.quote == asset_id)
	}
	pub fn base_asset_str(&self) -> String {
		match self.base {
			AssetId::polkadex => "PDEX".into(),
			AssetId::asset(id) => id.to_string(),
		}
	}
	pub fn quote_asset_str(&self) -> String {
		match self.quote {
			AssetId::polkadex => "PDEX".into(),
			AssetId::asset(id) => id.to_string(),
		}
	}
	pub fn market_id(&self) -> String {
		format!("{}/{}", self.base_asset_str(), self.quote_asset_str())
	}
}

#[cfg(feature = "std")]
impl Display for OrderSide {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			OrderSide::Ask => write!(f, "Ask"),
			OrderSide::Bid => write!(f, "Bid"),
		}
	}
}

#[cfg(feature = "std")]
impl Display for TradingPair {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{:}-{:}", self.base, self.quote)
	}
}

#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Order {
	pub stid: u64,
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
			return None
		}
		if self.side == OrderSide::Bid {
			// Buy side
			match self.price.cmp(&other.price) {
				// A.price < B.price => [B, A] (in buy side, the first prices should be the highest)
				Ordering::Less => Some(Ordering::Greater),
				// A.price == B.price =>  Order based on timestamp - lowest timestamp first
				Ordering::Equal => Some(self.timestamp.cmp(&other.timestamp)),
				// A.price > B.price => [A, B]
				Ordering::Greater => Some(Ordering::Less),
			}
		} else {
			// Sell side
			match self.price.cmp(&other.price) {
				// A.price < B.price => [A, B] (in sell side, the first prices should be the lowest)
				Ordering::Less => Some(Ordering::Less),
				// A.price == B.price => Order based on timestamp - lowest timestamp first
				Ordering::Equal => Some(self.timestamp.cmp(&other.timestamp)),
				// A.price > B.price => [B, A]
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
				// A.price < B.price => [B, A] (in buy side, the first prices should be the highest)
				Ordering::Less => Ordering::Greater,
				// A.price == B.price => Order based on timestamp
				Ordering::Equal => self.timestamp.cmp(&other.timestamp),
				// A.price > B.price => [A, B]
				Ordering::Greater => Ordering::Less,
			}
		} else {
			// Sell side
			match self.price.cmp(&other.price) {
				// A.price < B.price => [A, B] (in sell side, the first prices should be the lowest)
				Ordering::Less => Ordering::Less,
				// A.price == B.price => Order based on timestamp
				Ordering::Equal => self.timestamp.cmp(&other.timestamp),
				// A.price > B.price => [B, A]
				Ordering::Greater => Ordering::Greater,
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
			},
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
			)
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
	// TODO: how to gate this only for testing
	#[cfg(feature = "std")]
	pub fn random_order_for_testing(
		pair: TradingPair,
		side: OrderSide,
		order_type: OrderType,
	) -> Self {
		use rand::Rng;
		let mut rng = rand::thread_rng();
		Self {
			stid: Default::default(),
			client_order_id: H256([1u8; 32]),
			avg_filled_price: Decimal::zero(),
			fee: Decimal::zero(),
			filled_quantity: Decimal::zero(),
			status: OrderStatus::OPEN,
			id: H256([2u8; 32]),
			user: AccountId::new(rng.gen()),
			main_account: AccountId::new([0u8; 32]),
			pair,
			side,
			order_type,
			qty: Decimal::from(rng.gen_range(MIN_QTY..MAX_QTY)),
			price: Decimal::from(rng.gen_range(MIN_PRICE..MAX_PRICE)),
			quote_order_qty: Decimal::zero(),
			timestamp: 1,
			overall_unreserved_volume: Decimal::zero(),
		}
	}
}

pub fn rounding_off(a: Decimal) -> Decimal {
	// if we want to operate with a precision of 8 decimal places,
	// all calculations should be done with latest 9 decimal places
	a.round_dp_with_strategy(9, RoundingStrategy::ToZero)
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
