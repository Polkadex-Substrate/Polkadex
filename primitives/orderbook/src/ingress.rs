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

use crate::lmp::LMPEpochConfig;
use crate::{ocex::TradingPairConfig, AssetId};
use parity_scale_codec::{Decode, Encode};
use rust_decimal::Decimal;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::vec::Vec;

/// Definition of available ingress messages variants.
#[derive(
    Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize,
)]
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

    /// Trading Fees related
    WithdrawTradingFees,

    /// Liquidity Mining Variants
    /// Add Liquidity ( market, pool_id, LP, total Shares issued,  base_amount, quote_amount)
    AddLiquidity(
        TradingPairConfig,
        AccountId,
        AccountId,
        Decimal,
        Decimal,
        Decimal,
    ),
    /// Remove liquidity ( market, pool_id, LP,  burn_fraction, total_shares_issued_at_burn)
    RemoveLiquidity(TradingPairConfig, AccountId, AccountId, Decimal, Decimal),
    /// Force Close Command ( market, pool_id)
    ForceClosePool(TradingPairConfig, AccountId),
    /// LMPConfig
    LMPConfig(LMPEpochConfig),
    /// New LMP Epoch started
    NewLMPEpoch(u16),
}

#[serde_as]
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EgressMessages<AccountId> {
    /// Add Liquidity Result ( Pool, LP, Shares issued, Market price, total Inventory ( in Quote) )
    AddLiquidityResult(
        TradingPairConfig,
        AccountId,
        AccountId,
        Decimal,
        Decimal,
        Decimal,
    ),
    /// RemoveLiquidityResult ( Pool, LP, Base freed, Quote Freed )
    RemoveLiquidityResult(TradingPairConfig, AccountId, AccountId, Decimal, Decimal),
    /// Remove Liquidity Failed ( Pool, LP, burn_frac, total_shares_issued, base_free, quote_free,
    /// base_required, quote_required)
    RemoveLiquidityFailed(
        TradingPairConfig,
        AccountId,
        AccountId,
        Decimal,
        Decimal,
        Decimal,
        Decimal,
        Decimal,
        Decimal,
    ),
    /// Pool Closed (market, Pool, base freed, quote freed)
    PoolForceClosed(TradingPairConfig, AccountId, Decimal, Decimal),
    /// Trading Fees Collected
    TradingFees(#[serde_as(as = "Vec<(_, _)>")] BTreeMap<AssetId, Decimal>),
    /// Price Oracle
    PriceOracle(#[serde_as(as = "Vec<(_, _)>")] BTreeMap<(AssetId, AssetId), Decimal>),
}
