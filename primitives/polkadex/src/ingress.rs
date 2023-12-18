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

//! In this module defined ingress messages related types.

use std::collections::BTreeMap;
use crate::{ocex::TradingPairConfig, AssetId};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

/// Definition of available ingress messages variants.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum IngressMessages<AccountId> {
	/// Open Trading Pair.
	OpenTradingPair(TradingPairConfig),
	/// Update Trading Pair Config.
	UpdateTradingPair(TradingPairConfig),
	/// Register User ( main, proxy).
	RegisterUser(AccountId, AccountId),
	/// Main Acc, Assetid, Amount.
	Deposit(AccountId, AssetId, Decimal),
	/// Main Acc, Proxy Account.
	AddProxy(AccountId, AccountId),
	/// Main Acc, Proxy Account.
	RemoveProxy(AccountId, AccountId),
	/// Close Trading Pair.
	CloseTradingPair(TradingPairConfig),
	/// Changing the exchange state in order-book.
	SetExchangeState(bool),
	/// Withdrawal from Chain to OrderBook.
	DirectWithdrawal(AccountId, AssetId, Decimal, bool),
	/// Update Fee Structure ( main, maker_fraction, taker_fraction)
	UpdateFeeStructure(AccountId, Decimal, Decimal),


	/// Liquidity Mining Variants
	/// Add Liquidity ( market, pool_id, LP, total Shares issued,  base_amount, quote_amount)
	AddLiquidity(TradingPairConfig, AccountId, AccountId, Decimal, Decimal, Decimal),
	/// Remove liquidity ( market, pool_id, LP,  burn_fraction)
	RemoveLiquidity(TradingPairConfig,AccountId, AccountId, Decimal),
	/// Force Close Command ( market, pool_id)
	ForceClosePool(TradingPairConfig, AccountId)
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum EgressMessages<AccountId> {
	/// Add Liquidity Result ( Pool, LP, Shares issued, Market price, total Inventory ( in Quote) )
	AddLiquidityResult(AccountId, AccountId, Decimal, Decimal, Decimal),
	/// RemoveLiquidityResult ( Pool, LP, Base freed, Quote Freed )
	RemoveLiquidityResult(AccountId, AccountId, Decimal, Decimal),
	/// Remove Liquidity Failed ( Pool, LP, burn_frac, base_free, quote_free, base_required, quote_required)
	RemoveLiquidityFailed(AccountId, AccountId,Decimal, Decimal, Decimal, Decimal, Decimal),
	/// Pool Closed (market, Pool, base freed, quote freed)
	PoolForceClosed(TradingPairConfig,AccountId, Decimal, Decimal),
}

/// Defines the structure of handle balance data which used to set account balance.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HandleBalance<AccountId> {
	/// Main account identifier
	pub main_account: AccountId,
	/// Asset identifier.
	pub asset_id: AssetId,
	/// Operation fee.
	pub free: u128,
	/// Reserved amount.
	pub reserve: u128,
}

/// Defines a limit of the account handle balance.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HandleBalanceLimit;

impl Get<u32> for HandleBalanceLimit {
	/// Accessor to the handle balance limit amount.
	fn get() -> u32 {
		1000
	}
}

/// Defines a limit of the state hashes.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StateHashesLimit;

impl Get<u32> for StateHashesLimit {
	/// Accessor to the state hashes limit amount.
	/// For max 20 GB and 10 MB chunks.
	fn get() -> u32 {
		2000
	}
}
