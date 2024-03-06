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

use crate::types::TradingPair;
use frame_support::dispatch::DispatchResult;
use rust_decimal::Decimal;

pub trait LiquidityMiningCrowdSourcePallet<AccountId> {
	fn new_epoch(n: u16);
	fn add_liquidity_success(
		market: TradingPair,
		pool: &AccountId,
		lp: &AccountId,
		shared_issued: Decimal,
		price: Decimal,
		total_inventory_in_quote: Decimal,
	) -> DispatchResult;

	fn remove_liquidity_success(
		market: TradingPair,
		pool: &AccountId,
		lp: &AccountId,
		base_free: Decimal,
		quote_free: Decimal,
	) -> DispatchResult;

	#[allow(clippy::too_many_arguments)]
	fn remove_liquidity_failed(
		market: TradingPair,
		pool: &AccountId,
		lp: &AccountId,
		burn_frac: Decimal,
		total_shares: Decimal,
		base_free: Decimal,
		quote_free: Decimal,
		base_required: Decimal,
		quote_required: Decimal,
	) -> DispatchResult;

	fn pool_force_close_success(
		market: TradingPair,
		pool: &AccountId,
		base_freed: Decimal,
		quote_freed: Decimal,
	) -> DispatchResult;

	fn stop_accepting_lmp_withdrawals(epoch: u16);
}

impl<AccountId> LiquidityMiningCrowdSourcePallet<AccountId> for () {
	fn new_epoch(_n: u16) {}

	fn add_liquidity_success(
		_market: TradingPair,
		_pool: &AccountId,
		_lp: &AccountId,
		_shared_issued: Decimal,
		_price: Decimal,
		_total_inventory_in_quote: Decimal,
	) -> DispatchResult {
		Ok(())
	}

	fn remove_liquidity_success(
		_market: TradingPair,
		_pool: &AccountId,
		_lp: &AccountId,
		_base_free: Decimal,
		_quote_free: Decimal,
	) -> DispatchResult {
		Ok(())
	}

	fn remove_liquidity_failed(
		_market: TradingPair,
		_pool: &AccountId,
		_lp: &AccountId,
		_burn_frac: Decimal,
		_total_shares: Decimal,
		_base_free: Decimal,
		_quote_free: Decimal,
		_base_required: Decimal,
		_quote_required: Decimal,
	) -> DispatchResult {
		Ok(())
	}

	fn pool_force_close_success(
		_market: TradingPair,
		_pool: &AccountId,
		_base_freed: Decimal,
		_quote_freed: Decimal,
	) -> DispatchResult {
		Ok(())
	}

	fn stop_accepting_lmp_withdrawals(_epoch: u16) {}
}
