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

use crate::assets::AssetId;
use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use scale_info::TypeInfo;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{fees::FeeConfig, withdrawal::Withdrawal};

#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AccountInfo<Account, ProxyLimit: Get<u32>> {
	pub main_account: Account,
	pub proxies: BoundedVec<Account, ProxyLimit>,
	pub balances: BTreeMap<AssetId, (Decimal, Decimal)>,
	/// Trading Fee config
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
	pub fn new(main_account_id: Account) -> AccountInfo<Account, ProxyLimit> {
		let proxies = BoundedVec::default();
		AccountInfo {
			main_account: main_account_id,
			proxies,
			balances: BTreeMap::new(),
			fee_config: Default::default(),
		}
	}

	// Adds a new proxy account
	pub fn add_proxy(&mut self, proxy: Account) -> Result<(), Account> {
		self.proxies.try_push(proxy)
	}

	// Removes a proxy account
	pub fn remove_proxy(&mut self, proxy: &Account) {
		self.proxies.retain(|item| item != proxy);
	}
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct TradingPairConfig {
	pub base_asset: AssetId,
	pub quote_asset: AssetId,
	pub min_price: Decimal,
	pub max_price: Decimal,
	pub price_tick_size: Decimal,
	pub min_qty: Decimal,
	pub max_qty: Decimal,
	pub qty_step_size: Decimal,
	pub operational_status: bool, //will be true if the trading pair is enabled on the orderbook.
	pub base_asset_precision: u8,
	pub quote_asset_precision: u8,
}

impl TradingPairConfig {
	pub fn min_volume(&self) -> Decimal {
		self.min_qty.saturating_mul(self.min_price)
	}

	// This is an easy to use default config for testing and other purposes.
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

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum OnChainEvents<AccountId> {
	OrderBookWithdrawalClaimed(u64, AccountId, Vec<Withdrawal<AccountId>>),
	OrderbookWithdrawalProcessed(u64, Vec<Withdrawal<AccountId>>),
}
