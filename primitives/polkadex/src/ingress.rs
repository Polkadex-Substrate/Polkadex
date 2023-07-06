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

use crate::{ocex::TradingPairConfig, AssetId};
#[cfg(any(feature = "std", feature = "sgx"))]
use serde::{Deserialize, Serialize};

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::{traits::Get, BoundedVec};
use rust_decimal::Decimal;
use scale_info::TypeInfo;

/// Definition of available ingress messages variants.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(any(feature = "std", feature = "sgx"), derive(Serialize, Deserialize))]
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
}

/// Defines a limit of the account handle balance.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(any(feature = "std", feature = "sgx"), derive(Serialize, Deserialize))]
pub struct HandleBalanceLimit;

impl Get<u32> for HandleBalanceLimit {
	/// Accessor to the handle balance limit amount.
	fn get() -> u32 {
		1000
	}
}

/// Defines a limit of the state hashes.
#[derive(Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, Eq, PartialEq)]
#[cfg_attr(any(feature = "std", feature = "sgx"), derive(Serialize, Deserialize))]
pub struct StateHashesLimit;

impl Get<u32> for StateHashesLimit {
	/// Accessor to the state hashes limit amount.
	/// For max 20 GB and 10 MB chunks.
	fn get() -> u32 {
		2000
	}
}
