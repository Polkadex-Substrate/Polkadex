#[cfg(feature = "std")]
use std::{
	borrow::Borrow,
	fmt::{Display, Formatter},
	ops::{Mul, Rem},
	str::FromStr,
};

#[cfg(feature = "std")]
use chrono::Utc;
#[cfg(feature = "std")]
use libp2p::PeerId;
use parity_scale_codec::{Decode, Encode};
use rust_decimal::{
	prelude::{FromPrimitive, Zero},
	Decimal, RoundingStrategy,
};
use sp_core::H256;
use sp_runtime::traits::Verify;
use sp_std::cmp::Ordering;
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;

use polkadex_primitives::{
	ocex::TradingPairConfig, withdrawal::Withdrawal, AccountId, AssetId, Signature,
};

use crate::constants::*;

pub type OrderId = H256;

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

impl AccountAsset {
	pub fn new(main: AccountId, asset: AssetId) -> Self {
		AccountAsset { main, asset }
	}
}

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "std")]
pub struct Trade {
	pub maker: Order,
	pub taker: Order,
	pub price: Decimal,
	pub amount: Decimal,
	pub time: i64,
}

#[cfg(feature = "std")]
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
impl Trade {
	// Creates a Trade with zero event_tag
	pub fn new(maker: Order, taker: Order, price: Decimal, amount: Decimal) -> Trade {
		Self { maker, taker, price, amount, time: Utc::now().timestamp_millis() }
	}

	// Verifies the contents of a trade
	pub fn verify(&self, config: TradingPairConfig) -> bool {
		// Verify signatures
		self.maker.verify_signature() &
            self.taker.verify_signature() &
            // Verify pair configs
            self.maker.verify_config(&config) &
            self.taker.verify_config(&config)
	}
}

#[cfg(feature = "std")]
#[derive(Clone, Debug, Encode, Decode, serde::Serialize, serde::Deserialize)]
pub enum GossipMessage {
	/// (From, to)
	WantWorkerNonce(u64, u64),
	/// Collection of WorkerNonces
	WorkerNonces(Box<Vec<ObMessage>>),
	/// Single ObMessage
	ObMessage(Box<ObMessage>),
	/// Snapshot id, bitmap, remote peer
	Want(u64, Vec<u128>),
	/// Snapshot id, bitmap, remote peer
	Have(u64, Vec<u128>),
	/// Request
	/// (snapshot id, chunk indexes requested as bitmap)
	RequestChunk(u64, Vec<u128>),
	/// Chunks of snapshot data
	/// ( snapshot id, index of chunk, data )
	Chunk(u64, u16, Vec<u8>),
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "std")]
pub struct ObMessage {
	pub stid: u64,
	pub worker_nonce: u64,
	pub action: UserActions,
	pub signature: sp_core::ecdsa::Signature,
}

#[cfg(feature = "std")]
impl ObMessage {
	pub fn verify(&self, public_key: &sp_core::ecdsa::Public) -> bool {
		match self.signature.recover_prehashed(&self.sign_data()) {
			None => false,
			Some(recovered_pubk) => &recovered_pubk == public_key,
		}
	}

	pub fn sign_data(&self) -> [u8; 32] {
		let mut cloned_self = self.clone();
		cloned_self.signature = sp_core::ecdsa::Signature::default();
		sp_core::hashing::keccak_256(&cloned_self.encode())
	}
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(feature = "std")]
pub enum StateSyncStatus {
	Unavailable,
	// We don't have this chunk yet
	// (Who is supposed to send us, when we requested)
	InProgress(PeerId, i64),
	// We have asked a peer for this chunk and waiting
	Available, // We have this chunk
}

#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "std")]
pub enum UserActions {
	Trade(Vec<Trade>),
	Withdraw(WithdrawalRequest),
	BlockImport(u32),
}

#[derive(Clone, Debug, Decode, Encode, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "std")]
pub struct WithdrawalRequest {
	pub signature: Signature,
	pub payload: WithdrawPayloadCallByUser,
	pub main: AccountId,
	pub proxy: AccountId,
}

#[cfg(feature = "std")]
impl TryInto<Withdrawal<AccountId>> for WithdrawalRequest {
	type Error = rust_decimal::Error;

	fn try_into(self) -> Result<Withdrawal<AccountId>, rust_decimal::Error> {
		Ok(Withdrawal {
			main_account: self.main.clone(),
			amount: self.amount()?,
			asset: self.payload.asset_id,
			fees: Default::default(),
			stid: 0,
			worker_nonce: 0,
		})
	}
}

#[cfg(feature = "std")]
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

#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
impl TryFrom<String> for OrderSide {
	type Error = anyhow::Error;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		match value.as_str() {
			"Bid" => Ok(OrderSide::Bid),
			"Ask" => Ok(OrderSide::Ask),
			_ => Err(anyhow::Error::msg(format!("Unknown side variant: {value:?}"))),
		}
	}
}

#[derive(Encode, Decode, Copy, Clone, Hash, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderType {
	LIMIT,
	MARKET,
}

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
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

#[cfg(feature = "std")]
impl From<OrderStatus> for String {
	fn from(value: OrderStatus) -> Self {
		match value {
			OrderStatus::OPEN => "OPEN".to_string(),
			OrderStatus::CLOSED => "CLOSED".to_string(),
			OrderStatus::CANCELLED => "CANCELLED".to_string(),
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
			AssetId::Polkadex
		} else {
			let id = assets[0].parse::<u128>()?;
			AssetId::Asset(id)
		};

		let quote_asset = if assets[1] == String::from("PDEX").as_str() {
			AssetId::Polkadex
		} else {
			let id = assets[1].parse::<u128>()?;
			AssetId::Asset(id)
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
	#[cfg(feature = "std")]
	pub fn base_asset_str(&self) -> String {
		match self.base {
			AssetId::Polkadex => "PDEX".into(),
			AssetId::Asset(id) => id.to_string(),
		}
	}
	#[cfg(feature = "std")]
	pub fn quote_asset_str(&self) -> String {
		match self.quote {
			AssetId::Polkadex => "PDEX".into(),
			AssetId::Asset(id) => id.to_string(),
		}
	}

	#[cfg(feature = "std")]
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
	pub signature: Signature,
}

#[cfg(feature = "std")]
impl Order {
	pub fn verify_config(&self, config: &TradingPairConfig) -> bool {
		let is_market_same =
			self.pair.base == config.base_asset && self.pair.quote == config.quote_asset;
		let result = match self.order_type {
			OrderType::LIMIT =>
				is_market_same &&
					self.price >= config.min_price &&
					self.price <= config.max_price &&
					self.qty >= config.min_qty &&
					self.qty <= config.max_qty &&
					self.price.rem(config.price_tick_size).is_zero() &&
					self.qty.rem(config.qty_step_size).is_zero(),
			OrderType::MARKET =>
				if self.side == OrderSide::Ask {
					// for ask order we are checking base order qty
					is_market_same &&
						self.qty >= config.min_qty &&
						self.qty <= config.max_qty &&
						self.qty.rem(config.qty_step_size).is_zero()
				} else {
					// for bid order we are checking quote order qty
					is_market_same &&
						self.quote_order_qty >= (config.min_qty * config.min_price) &&
						self.quote_order_qty <= (config.max_qty * config.max_price) &&
						self.quote_order_qty.rem(config.price_tick_size).is_zero()
				},
		};
		if !result {
			log::error!(target:"orderbook","pair config verification failed: config: {:?}, price: {:?}, qty: {:?}, quote_order_qty: {:?}", config, self.price, self.qty, self.quote_order_qty);
		}
		result
	}

	pub fn verify_signature(&self) -> bool {
		let payload: OrderPayload = self.clone().into();
		let result = self.signature.verify(&payload.encode()[..], &self.user);
		if !result {
			log::error!(target:"orderbook","Order signature check failed");
		}
		result
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
				Ordering::Less => Some(Ordering::Less),
				// A.price == B.price =>  Order based on timestamp - lowest timestamp first
				Ordering::Equal =>
					if self.timestamp < other.timestamp {
						Some(Ordering::Greater)
					} else {
						Some(Ordering::Less)
					},
				// A.price > B.price => [A, B]
				Ordering::Greater => Some(Ordering::Greater),
			}
		} else {
			// Sell side
			match self.price.cmp(&other.price) {
				// A.price < B.price => [A, B] (in sell side, the first prices should be the lowest)
				Ordering::Less => Some(Ordering::Greater),
				// A.price == B.price => Order based on timestamp - lowest timestamp first
				Ordering::Equal => {
					// If price is equal, we follow the FIFO priority
					if self.timestamp < other.timestamp {
						Some(Ordering::Greater)
					} else {
						Some(Ordering::Less)
					}
				},
				// A.price > B.price => [B, A]
				Ordering::Greater => Some(Ordering::Less),
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
				Ordering::Less => Ordering::Less,
				// A.price == B.price => Order based on timestamp
				Ordering::Equal =>
					if self.timestamp < other.timestamp {
						Ordering::Greater
					} else {
						Ordering::Less
					},
				// A.price > B.price => [A, B]
				Ordering::Greater => Ordering::Greater,
			}
		} else {
			// Sell side
			match self.price.cmp(&other.price) {
				// A.price < B.price => [A, B] (in sell side, the first prices should be the lowest)
				Ordering::Less => Ordering::Greater,
				// A.price == B.price => Order based on timestamp
				Ordering::Equal => {
					// If price is equal, we follow the FIFO priority
					if self.timestamp < other.timestamp {
						Ordering::Greater
					} else {
						Ordering::Less
					}
				},
				// A.price > B.price => [B, A]
				Ordering::Greater => Ordering::Less,
			}
		}
	}
}

#[cfg(feature = "std")]
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
				println!("Quotient: {quotient:?}");
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
			signature: Signature::Sr25519(sp_core::sr25519::Signature::from_raw([0; 64])),
		}
	}
}

pub fn rounding_off(a: Decimal) -> Decimal {
	// if we want to operate with a precision of 8 decimal places,
	// all calculations should be done with latest 9 decimal places
	a.round_dp_with_strategy(9, RoundingStrategy::ToZero)
}

#[cfg(feature = "std")]
pub struct OrderDetails {
	pub payload: OrderPayload,
	pub signature: Signature,
}

#[derive(Encode, Decode, Clone, Debug, serde::Serialize, serde::Deserialize)]
#[cfg(feature = "std")]
pub struct OrderPayload {
	pub client_order_id: H256,
	pub user: AccountId,
	pub main_account: AccountId,
	pub pair: String,
	pub side: OrderSide,
	pub order_type: OrderType,
	pub quote_order_quantity: String,
	// Quantity is defined in base asset
	pub qty: String,
	// Price is defined in quote asset per unit base asset
	pub price: String,
	pub timestamp: i64,
}

#[cfg(feature = "std")]
impl From<Order> for OrderPayload {
	fn from(value: Order) -> Self {
		Self {
			client_order_id: value.client_order_id,
			user: value.user,
			main_account: value.main_account,
			pair: value.pair.to_string(),
			side: value.side,
			order_type: value.order_type,
			quote_order_quantity: value.quote_order_qty.to_string(),
			qty: value.qty.to_string(),
			price: value.price.to_string(),
			timestamp: value.timestamp,
		}
	}
}

#[cfg(feature = "std")]
impl TryFrom<OrderDetails> for Order {
	type Error = anyhow::Error;
	fn try_from(details: OrderDetails) -> Result<Self, anyhow::Error> {
		let payload = details.payload;
		if let Ok(qty) = payload.qty.parse::<f64>() {
			if let Ok(price) = payload.price.parse::<f64>() {
				return if let Some(qty) = Decimal::from_f64(qty) {
					if let Some(price) = Decimal::from_f64(price) {
						if let Ok(quote_order_qty) = payload.quote_order_quantity.parse::<f64>() {
							if let Some(quote_order_qty) = Decimal::from_f64(quote_order_qty) {
								if let Ok(trading_pair) = payload.pair.try_into() {
									Ok(Self {
										stid: 0,
										client_order_id: payload.client_order_id,
										avg_filled_price: Decimal::zero(),
										fee: Decimal::zero(),
										filled_quantity: Decimal::zero(),
										id: H256::random(),
										status: OrderStatus::OPEN,
										user: payload.user,
										main_account: payload.main_account,
										pair: trading_pair,
										side: payload.side,
										order_type: payload.order_type,
										qty: qty.round_dp(8),
										price: price.round_dp(8),
										quote_order_qty: quote_order_qty.round_dp(8),
										timestamp: payload.timestamp,
										overall_unreserved_volume: Decimal::zero(),
										signature: details.signature,
									})
								} else {
									Err(anyhow::Error::msg(
										"Not able to to parse trading pair".to_string(),
									))
								}
							} else {
								Err(anyhow::Error::msg(
									"Quote order quantity couldn't be parsed to decimal"
										.to_string(),
								))
							}
						} else {
							Err(anyhow::Error::msg(
								"Quote order quantity couldn't be parsed".to_string(),
							))
						}
					} else {
						Err(anyhow::Error::msg(
							"Price couldn't be converted to decimal".to_string(),
						))
					}
				} else {
					Err(anyhow::Error::msg("Qty couldn't be converted to decimal".to_string()))
				}
			}
			return Err(anyhow::Error::msg("Price couldn't be parsed".to_string()))
		}
		Err(anyhow::Error::msg(format!("Qty couldn't be parsed {}", payload.qty)))
	}
}

#[cfg(feature = "std")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, Encode, Decode, Eq, PartialEq)]
pub struct WithdrawalDetails {
	pub payload: WithdrawPayloadCallByUser,
	pub main: AccountId,
	pub proxy: AccountId,
	pub signature: Signature,
}

#[cfg(test)]
mod tests {
	use crate::types::{ObMessage, UserActions};

	#[test]
	pub fn test_ob_message() {
		let msg = ObMessage {
			stid: 0,
			worker_nonce: 0,
			action: UserActions::BlockImport(1),
			signature: Default::default(),
		};

		println!("OBMessage: {:?}", serde_json::to_string(&msg).unwrap());
	}
}
