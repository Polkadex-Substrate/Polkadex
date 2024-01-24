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

//! This module contains "OCEX" pallet related primitives.

use crate::assets::AssetId;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use scale_info::TypeInfo;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{fees::FeeConfig, withdrawal::Withdrawal};

/// Account related information structure definition required for users registration and storage.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AccountInfo<Account, ProxyLimit: Get<u32>> {
	/// Main account.
	pub main_account: Account,
	/// Proxy accounts.
	pub proxies: BoundedVec<Account, ProxyLimit>,
	/// Account balances.
	pub balances: BTreeMap<AssetId, (Decimal, Decimal)>,
	/// Trading Fee configuration.
	pub fee_config: FeeConfig,
}

impl<Account: PartialEq, ProxyLimit: Get<u32>> AccountInfo<Account, ProxyLimit> {
	pub fn maker_fee_fraction(&self) -> Decimal {
		self.fee_config.maker_fraction
	}
	pub fn taker_fee_fraction(&self) -> Decimal {
		self.fee_config.taker_fraction
	}
}

impl<Account: PartialEq, ProxyLimit: Get<u32>> AccountInfo<Account, ProxyLimit> {
	/// Constructor.
	///
	/// # Parameters
	///
	/// * `main_account_id`: Main account identifier.
	pub fn new(main_account_id: Account) -> AccountInfo<Account, ProxyLimit> {
		let proxies = BoundedVec::default();
		AccountInfo {
			main_account: main_account_id,
			proxies,
			balances: BTreeMap::new(),
			fee_config: Default::default(),
		}
	}

	/// Adds a new proxy account.
	///
	/// # Parameters
	///
	/// * `proxy`: Proxy account identifier.
	pub fn add_proxy(&mut self, proxy: Account) -> Result<(), Account> {
		self.proxies.try_push(proxy)
	}

	/// Removes a proxy account.
	///
	/// # Parameters
	///
	/// * `proxy`: Proxy account identifier.
	pub fn remove_proxy(&mut self, proxy: &Account) {
		self.proxies.retain(|item| item != proxy);
	}
}

/// Trading pair configuration structure definition.
#[derive(
	Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq, Copy, Ord, PartialOrd,
)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPairConfig {
	/// Base asset identifier.
	pub base_asset: AssetId,
	/// Quote asset identifier.
	pub quote_asset: AssetId,
	/// Minimum trading price.
	pub min_price: Decimal,
	/// Maximum trading price.
	pub max_price: Decimal,
	/// The minimum price increment change.
	pub price_tick_size: Decimal,
	/// Minimum quantity.
	pub min_qty: Decimal,
	/// Maximum quantity.
	pub max_qty: Decimal,
	/// The minimum quantity increment change.
	pub qty_step_size: Decimal,
	/// Defines if trading operation is enabled or disabled.
	///
	/// Will be true if the trading pair is enabled on the orderbook.
	pub operational_status: bool,
	/// Base asset precision.
	pub base_asset_precision: u8,
	/// Quote asset precision.
	pub quote_asset_precision: u8,
}

impl TradingPairConfig {
	/// Minimum appropriate trading volume.
	pub fn min_volume(&self) -> Decimal {
		self.min_qty.saturating_mul(self.min_price)
	}

	/// This is an easy to use default config for testing and other purposes.
	pub fn default(base: AssetId, quote: AssetId) -> Self {
		Self {
			base_asset: base,
			quote_asset: quote,
			min_price: Decimal::from_f64(0.0001).unwrap(),
			max_price: Decimal::from_f64(1000.0).unwrap(),
			price_tick_size: Decimal::from_f64(0.000001).unwrap(),
			min_qty: Decimal::from_f64(0.0001).unwrap(),
			max_qty: Decimal::from_f64(1000.0).unwrap(),
			qty_step_size: Decimal::from_f64(0.001).unwrap(),
			operational_status: true,
			base_asset_precision: 8,
			quote_asset_precision: 8,
		}
	}
}

/// Defines possible "onchain" events.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OnChainEvents<AccountId> {
	/// Withdrawal claimed. (Snapshot id, Account id, Collection of withdrawals).
	OrderBookWithdrawalClaimed(u64, AccountId, Vec<Withdrawal<AccountId>>),
	/// Withdrawal processed. (Snapshot id, Collection of withdrawals).
	OrderbookWithdrawalProcessed(u64, Vec<Withdrawal<AccountId>>),
}
