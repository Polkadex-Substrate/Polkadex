// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! In this module defined "Orderbook" specific operations and types.

use crate::constants::*;
use parity_scale_codec::{Codec, Decode, Encode};
use polkadex_primitives::{
	ocex::TradingPairConfig, withdrawal::Withdrawal, AccountId, AssetId, Signature,
};
use rust_decimal::{prelude::Zero, Decimal, RoundingStrategy};
use sp_core::H256;
use sp_runtime::traits::Verify;
use sp_std::cmp::Ordering;

#[cfg(not(feature = "std"))]
use sp_std::fmt::{Display, Formatter};
#[cfg(not(feature = "std"))]
use sp_std::vec::Vec;
#[cfg(feature = "std")]
use std::{
	fmt::{Display, Formatter},
	ops::{Mul, Rem},
	str::FromStr,
};

pub type OrderId = H256;

/// Defined account information required for the "Orderbook" client.
#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountInfo {
	/// Collection of the proxy accounts.
	pub proxies: Vec<AccountId>,
}

/// Defines account to asset map DTO to be used in the "Orderbook" client.
#[derive(Clone, Debug, Encode, Decode, Ord, PartialOrd, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountAsset {
	/// Main account identifier.
	pub main: AccountId,
	/// Asset identifier.
	pub asset: AssetId,
}

impl AccountAsset {
	/// Constructor.
	///
	/// # Parameters
	///
	/// * `main`: Main account identifier.
	/// * `asset`: Asset identifier.
	pub fn new(main: AccountId, asset: AssetId) -> Self {
		AccountAsset { main, asset }
	}
}

/// Defines trade related structure DTO.
#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Trade {
	/// Market order.
	pub maker: Order,
	/// Taker order.
	pub taker: Order,
	/// Price of the trade.
	pub price: Decimal,
	/// Amount of the trade.
	pub amount: Decimal,
	/// Timestamp of the trade.
	pub time: i64,
}

impl Trade {
	/// Depends on the trade side - calculates and provides price and asset information required for
	/// further balances transfers.
	///
	/// # Parameters
	///
	/// * `market`: Defines if order is a market order.
	pub fn credit(&self, maker: bool) -> (AccountAsset, Decimal) {
		let user = if maker { &self.maker } else { &self.taker };
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

	/// Depends on the trade side - calculates and provides price and asset information required for
	/// further balances transfers.
	///
	/// # Parameters
	///
	/// * `market`: Defines if order is a market order.
	pub fn debit(&self, maker: bool) -> (AccountAsset, Decimal) {
		let user = if maker { &self.maker } else { &self.taker };
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
use rust_decimal::prelude::FromPrimitive;
use scale_info::TypeInfo;

impl Trade {
	/// Constructor.
	/// Creates a Trade with zero event_tag.
	///
	/// # Parameters
	///
	/// * `market`: Market order.
	/// * `taker`: Taker order.
	/// * `price`: Price of the trade.
	/// * `amount`: Amount of the trade.
	#[cfg(feature = "std")]
	pub fn new(maker: Order, taker: Order, price: Decimal, amount: Decimal) -> Trade {
		Self { maker, taker, price, amount, time: Utc::now().timestamp_millis() }
	}

	/// Verifies content of the trade.
	///
	/// # Parameters
	///
	/// * `config`: Trading pair configuration DTO.
	pub fn verify(&self, config: TradingPairConfig) -> bool {
		// Verify signatures
		self.maker.verify_signature() &
            self.taker.verify_signature() &
            // Verify pair configs
            self.maker.verify_config(&config) &
            self.taker.verify_config(&config)
	}
}

/// Defines "Orderbook" message structure DTO.
#[derive(Clone, Debug, Encode, Decode)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
#[cfg(feature = "std")]
pub struct ObMessage {
	/// State change identifier.
	pub stid: u64,
	/// Worker nonce.
	pub worker_nonce: u64,
	/// Specific action.
	pub action: UserActions<AccountId>,
	/// Ecdsa signature.
	pub signature: sp_core::ecdsa::Signature,
	pub reset: bool,
	pub version: u16,
}

/// A batch of user actions
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct UserActionBatch<AccountId: Clone + Codec + TypeInfo> {
	pub actions: Vec<UserActions<AccountId>>,
	pub stid: u64,
	pub snapshot_id: u64,
}

#[cfg(feature = "std")]
impl ObMessage {
	/// Verifies itself.
	///
	/// # Parameters
	///
	/// * `public_key`: Ecdsa public key.
	pub fn verify(&self, public_key: &sp_core::ecdsa::Public) -> bool {
		match self.signature.recover_prehashed(&self.sign_data()) {
			None => false,
			Some(recovered_pubk) => &recovered_pubk == public_key,
		}
	}

	/// Signs itself.
	pub fn sign_data(&self) -> [u8; 32] {
		let mut cloned_self = self.clone();
		cloned_self.signature = sp_core::ecdsa::Signature::default();
		sp_core::hashing::keccak_256(&cloned_self.encode())
	}
}

/// Defines user specific operations variants.
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum UserActions<AccountId: Codec + Clone + TypeInfo> {
	/// Trade operation requested.
	Trade(Vec<Trade>),
	/// Withdraw operation requested.
	Withdraw(WithdrawalRequest<AccountId>),
	/// Block import requested.
	BlockImport(u32),
	/// Reset Flag
	Reset,
}

/// Defines withdraw request DTO.
#[derive(Clone, Debug, Decode, Encode, TypeInfo, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalRequest<AccountId: Codec + Clone + TypeInfo> {
	/// Signature.
	pub signature: Signature,
	/// Payload.
	pub payload: WithdrawPayloadCallByUser,
	/// User's main account identifier.
	pub main: AccountId,
	/// User's proxy account identifier.
	pub proxy: AccountId,
}

impl<AccountId: Clone + Codec + TypeInfo> WithdrawalRequest<AccountId> {
	pub fn convert(&self, stid: u64) -> Result<Withdrawal<AccountId>, rust_decimal::Error> {
		Ok(Withdrawal {
			main_account: self.main.clone(),
			amount: self.amount()?,
			asset: self.payload.asset_id,
			fees: Default::default(),
			stid,
		})
	}
}

impl<AccountId: Codec + Clone + TypeInfo> WithdrawalRequest<AccountId> {
	/// Verifies request payload.
	pub fn verify(&self) -> bool {
		let signer = match Decode::decode(&mut &self.proxy.encode()[..]) {
			Ok(signer) => signer,
			Err(_) => return false,
		};
		self.signature.verify(self.payload.encode().as_ref(), &signer)
	}

	/// Instantiates `AccountAsset` DTO based on owning data.
	pub fn asset(&self) -> AssetId {
		self.payload.asset_id
	}

	/// Tries to convert owning payload amount `String` value to `Decimal`.
	pub fn amount(&self) -> Result<Decimal, rust_decimal::Error> {
		Decimal::from_str(&self.payload.amount)
	}
}
#[cfg(not(feature = "std"))]
use core::{
	ops::{Mul, Rem},
	str::FromStr,
};
use parity_scale_codec::alloc::string::ToString;
use scale_info::prelude::string::String;

/// Withdraw payload requested by user.
#[derive(Encode, Decode, Clone, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawPayloadCallByUser {
	/// Asset identifier.
	pub asset_id: AssetId,
	/// Amount in a `String` representation.
	pub amount: String,
	/// Timestamp of the request.
	pub timestamp: i64,
}

/// Defines possible order sides variants.
#[derive(Encode, Decode, Copy, Clone, Hash, Ord, PartialOrd, Debug, Eq, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderSide {
	/// Asking order side.
	Ask,
	/// Bidding order side.
	Bid,
}

impl OrderSide {
	/// Resolves an opposite side of the current order side.
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

/// Defines possible order types variants.
#[derive(Encode, Decode, Copy, Clone, Hash, Debug, Eq, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderType {
	/// Order limit type.
	LIMIT,
	/// Order market type.
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

/// Defines possible order statuses variants.
#[derive(Encode, Decode, Copy, Clone, Hash, Debug, Eq, PartialEq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum OrderStatus {
	/// Order open.
	OPEN,
	/// Order closed.
	CLOSED,
	/// Order canceled.
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

/// Defines trading pair structure.
#[derive(Encode, Decode, Copy, Hash, Ord, PartialOrd, Clone, PartialEq, Debug, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct TradingPair {
	/// Base asset identifier.
	pub base: AssetId,
	/// Quote asset identifier.
	pub quote: AssetId,
}

impl TryFrom<String> for TradingPair {
	type Error = &'static str;
	fn try_from(value: String) -> Result<Self, Self::Error> {
		let assets: Vec<&str> = value.split('-').collect();
		if assets.len() != 2 {
			return Err("Invalid String")
		}

		let base_asset = if assets[0] == String::from("PDEX").as_str() {
			AssetId::Polkadex
		} else {
			let id = assets[0].parse::<u128>().map_err(|_| "asset id parse error")?;
			AssetId::Asset(id)
		};

		let quote_asset = if assets[1] == String::from("PDEX").as_str() {
			AssetId::Polkadex
		} else {
			let id = assets[1].parse::<u128>().map_err(|_| "asset id parse error")?;
			AssetId::Asset(id)
		};

		Ok(TradingPair::from(quote_asset, base_asset))
	}
}

impl TradingPair {
	/// Constructor.
	///
	/// # Parameters
	///
	/// * `quote`: Quote asset identifier.
	/// * `base`: Base asset identifier.
	pub fn from(quote: AssetId, base: AssetId) -> Self {
		TradingPair { base, quote }
	}

	/// Defines if provided asset is a quote asset of the current trading pair.
	///
	/// # Parameters
	///
	/// * `asset_id`: Asset identifier to compare.
	pub fn is_quote_asset(&self, asset_id: AssetId) -> bool {
		self.quote == asset_id
	}

	/// Defines if provided asset is a base asset of the current trading pair.
	///
	/// # Parameters
	///
	/// * `asset_id`: Asset identifier to compare.
	pub fn is_base_asset(&self, asset_id: AssetId) -> bool {
		self.base == asset_id
	}

	/// Defines if provided asset identifier is matching internal base or quote asset identifier.
	///
	/// # Parameters
	///
	/// * `asset_id`: Asset identifier.
	pub fn is_part_of(&self, asset_id: AssetId) -> bool {
		(self.base == asset_id) | (self.quote == asset_id)
	}

	/// Converts base asset identifier to the `String`.
	#[cfg(feature = "std")]
	pub fn base_asset_str(&self) -> String {
		match self.base {
			AssetId::Polkadex => "PDEX".into(),
			AssetId::Asset(id) => id.to_string(),
		}
	}

	/// Converts quote asset identifier to the `String`.
	#[cfg(feature = "std")]
	pub fn quote_asset_str(&self) -> String {
		match self.quote {
			AssetId::Polkadex => "PDEX".into(),
			AssetId::Asset(id) => id.to_string(),
		}
	}

	/// Normalizes base and quote assets to the market identifier.
	#[cfg(feature = "std")]
	pub fn market_id(&self) -> String {
		format!("{}/{}", self.base_asset_str(), self.quote_asset_str())
	}
}

impl Display for OrderSide {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		match self {
			OrderSide::Ask => write!(f, "Ask"),
			OrderSide::Bid => write!(f, "Bid"),
		}
	}
}

impl Display for TradingPair {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		write!(f, "{:}-{:}", self.base, self.quote)
	}
}

/// Order structure definition.
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Order {
	/// State change identifier.
	pub stid: u64,
	/// Client order identifier.
	pub client_order_id: H256,
	/// Average filled price.
	pub avg_filled_price: Decimal,
	/// Fee.
	pub fee: Decimal,
	/// Filled quantity.
	pub filled_quantity: Decimal,
	/// Status.
	pub status: OrderStatus,
	/// Identifier.
	pub id: OrderId,
	/// User's account identifier.
	pub user: AccountId,
	/// Main account identifier.
	pub main_account: AccountId,
	/// Trading pair.
	pub pair: TradingPair,
	/// Side of the order.
	pub side: OrderSide,
	/// Type.
	pub order_type: OrderType,
	/// Quantity.
	pub qty: Decimal,
	/// Price.
	pub price: Decimal,
	/// Quote order quantity.
	pub quote_order_qty: Decimal,
	/// Creation timestamp.
	pub timestamp: i64,
	/// Overall unreserved volume.
	pub overall_unreserved_volume: Decimal,
	/// Signature.
	pub signature: Signature,
}

impl Order {
	/// Verifies provided trading pair configuration.
	///
	/// # Parameters
	///
	/// * `config`: Trading pair configuration reference.
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

	/// Verifies signature.
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
	/// Computes the new avg_price and adds qty to filled_qty. If returned is false - then underflow
	/// occurred during division.
	///
	/// # Parameters
	///
	/// * `price`: New price.
	/// * `amount`: New amount.
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

	/// Calculates available volume.
	///
	/// # Parameters
	///
	/// * `other_price`: Optional price.
	pub fn available_volume(&self, other_price: Option<Decimal>) -> Decimal {
		//this if for market bid order
		if self.qty.is_zero() {
			println!(
				"quote_order_qty: {:?}, avg_filled_price: {:?}, filled_quantity: {:?}",
				self.quote_order_qty, self.avg_filled_price, self.filled_quantity
			);
			return Self::rounding_off(
				self.quote_order_qty
					.saturating_sub(self.avg_filled_price.saturating_mul(self.filled_quantity)),
			)
		}
		//this is for market ask order
		if self.order_type == OrderType::MARKET {
			Self::rounding_off(
				self.qty
					.saturating_sub(self.filled_quantity)
					.saturating_mul(other_price.unwrap_or_default()),
			)
		}
		//this is for limit orders
		else {
			// We cannot use avg. price here as limit orders might not have avg_price defined
			// if they are not yet matched and just inserted into the book
			Self::rounding_off(
				self.qty.saturating_sub(self.filled_quantity).saturating_mul(self.price),
			)
		}
	}

	pub fn rounding_off(a: Decimal) -> Decimal {
		// if we want to operate with a precision of 8 decimal places,
		// all calculations should be done with latest 9 decimal places
		a.round_dp_with_strategy(9, RoundingStrategy::ToZero)
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

/// Defines order details structure DTO.
pub struct OrderDetails {
	/// Payload of the order.
	pub payload: OrderPayload,
	/// Signature of the order.
	pub signature: Signature,
}

/// Defines payload of the order.
#[derive(Encode, Decode, Clone, Debug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct OrderPayload {
	/// Client order identifier.
	pub client_order_id: H256,
	/// User's account identifier.
	pub user: AccountId,
	/// Main account identifier.
	pub main_account: AccountId,
	/// Trading pair.
	pub pair: String,
	/// Side of the order.
	pub side: OrderSide,
	/// Type.
	pub order_type: OrderType,
	/// Quote order quantity.
	pub quote_order_quantity: String,
	/// Quantity.
	/// Quantity is defined in base asset.
	pub qty: String,
	/// Price.
	/// Price is defined in quote asset per unit base asset.
	pub price: String,
	/// Creation timestamp.
	pub timestamp: i64,
}

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
	type Error = &'static str;
	fn try_from(details: OrderDetails) -> Result<Self, Self::Error> {
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
									Err("Not able to to parse trading pair")
								}
							} else {
								Err("Quote order quantity couldn't be parsed to decimal")
							}
						} else {
							Err("Quote order quantity couldn't be parsed")
						}
					} else {
						Err("Price couldn't be converted to decimal")
					}
				} else {
					Err("Qty couldn't be converted to decimal")
				}
			}
			return Err("Price couldn't be parsed")
		}
		Err("Qty could not be parsed")
	}
}

/// Defines withdraw details DTO.
#[derive(Clone, Debug, Encode, Decode, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct WithdrawalDetails {
	/// Withdraw payload.
	pub payload: WithdrawPayloadCallByUser,
	/// Main account identifier.
	pub main: AccountId,
	/// Proxy account identifier.
	pub proxy: AccountId,
	/// Signature.
	pub signature: Signature,
}

#[cfg(test)]
mod tests {
	use crate::{
		types::{ApprovedSnapshot, ObMessage, UserActions},
		SnapshotSummary,
	};
	use parity_scale_codec::Encode;
	use polkadex_primitives::AccountId;
	use primitive_types::H256;
	use sp_core::Pair;

	#[test]
	pub fn test_ob_message() {
		let msg = ObMessage {
			stid: 0,
			worker_nonce: 0,
			action: UserActions::BlockImport(1),
			signature: Default::default(),
			reset: false,
			version: 0,
		};

		println!("OBMessage: {:?}", serde_json::to_string(&msg).unwrap());
	}

	#[test]
	pub fn approved_snapshot() {
		let pair = sp_core::sr25519::Pair::generate().0;
		let summary: SnapshotSummary<AccountId> = SnapshotSummary {
			validator_set_id: 1,
			snapshot_id: 1,
			state_hash: H256::random(),
			state_change_id: 1,
			last_processed_blk: 1,
			withdrawals: vec![],
		};

		let approved = ApprovedSnapshot {
			summary: summary.encode(),
			index: 1,
			signature: pair.sign(&summary.encode()).encode(),
		};

		println!("{:?}", serde_json::to_string(&approved).unwrap());
	}
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ApprovedSnapshot {
	pub summary: Vec<u8>,
	pub index: u16,
	pub signature: Vec<u8>,
}
