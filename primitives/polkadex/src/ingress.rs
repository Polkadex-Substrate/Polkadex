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

use crate::{ocex::TradingPairConfig, AssetId};
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum IngressMessages<AccountId> {
	// Open Trading Pair
	OpenTradingPair(TradingPairConfig),
	// Update Trading Pair Config
	UpdateTradingPair(TradingPairConfig),
	// Register User ( main, proxy)
	RegisterUser(AccountId, AccountId),
	// Main Acc, Assetid, Amount
	Deposit(AccountId, AssetId, Decimal),
	// Main Acc, Proxy Account
	AddProxy(AccountId, AccountId),
	// Main Acc, Proxy Account
	RemoveProxy(AccountId, AccountId),
	// Close Trading Pair
	CloseTradingPair(TradingPairConfig),
	// Resetting the balances of Account
	SetFreeReserveBalanceForAccounts(BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit>),
	// Changing the exchange state in order-book
	SetExchangeState(bool),
	// Withdrawal from Chain to OrderBook
	DirectWithdrawal(AccountId, AssetId, Decimal, bool),
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HandleBalance<AccountId> {
	pub main_account: AccountId,
	pub asset_id: AssetId,
	pub free: u128,
	pub reserve: u128,
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct HandleBalanceLimit;

impl Get<u32> for HandleBalanceLimit {
	fn get() -> u32 {
		1000
	}
}

#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct StateHashesLimit;

impl Get<u32> for StateHashesLimit {
	// for max 20 GB and 10 MB chunks
	fn get() -> u32 {
		2000
	}
}
