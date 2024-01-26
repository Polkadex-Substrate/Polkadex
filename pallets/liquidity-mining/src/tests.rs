// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

//! Tests for pallet-lmp.

use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use orderbook_primitives::{constants::UNIT_BALANCE, types::TradingPair};
use polkadex_primitives::AssetId;
use sp_core::crypto::AccountId32;
use std::{collections::BTreeMap, ops::DivAssign};

#[test]
fn test_register_pool_happy_path() {
	new_test_ext().execute_with(|| {
		// Register market OCEX
		let name = [1; 10];
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let commission = UNIT_BALANCE * 1;
		let exit_fee = UNIT_BALANCE * 1;
		let public_fund_allowed = true;
		let trading_account = AccountId32::new([1; 32]);
		let market_maker = AccountId32::new([2; 32]);
		register_test_trading_pair();
		mint_base_quote_asset_for_user(market_maker.clone());
		assert_ok!(LiqudityMining::register_pool(
			RuntimeOrigin::signed(market_maker.clone()),
			name,
			trading_pair,
			commission,
			exit_fee,
			public_fund_allowed,
			trading_account.clone()
		));
		// Verification
		assert!(LiqudityMining::lmp_pool(trading_pair, market_maker.clone()).is_some());
		assert_noop!(
			LiqudityMining::register_pool(
				RuntimeOrigin::signed(market_maker.clone()),
				name,
				trading_pair,
				commission,
				exit_fee,
				public_fund_allowed,
				trading_account
			),
			crate::pallet::Error::<Test>::PoolExists
		);
	})
}

#[test]
fn test_register_pool_error_unknown_pool() {
	new_test_ext().execute_with(|| {
		let name = [1; 10];
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let commission = UNIT_BALANCE * 1;
		let exit_fee = UNIT_BALANCE * 1;
		let public_fund_allowed = true;
		let trading_account = AccountId32::new([1; 32]);
		let market_maker = AccountId32::new([2; 32]);
		mint_base_quote_asset_for_user(market_maker.clone());
		assert_noop!(
			LiqudityMining::register_pool(
				RuntimeOrigin::signed(market_maker.clone()),
				name,
				trading_pair,
				commission,
				exit_fee,
				public_fund_allowed,
				trading_account.clone()
			),
			crate::pallet::Error::<Test>::UnknownMarket
		);
	})
}

use crate::pallet::{Pools};
use frame_support::traits::fungibles::Inspect;
use rust_decimal::{prelude::FromPrimitive, Decimal};

#[test]
fn test_register_pool_error_register_pool_fails() {
	new_test_ext().execute_with(|| {
		let main_account = AccountId32::new([1; 32]);
		let trading_account = AccountId32::new([2; 32]);
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_user(main_account, trading_account));
		let name = [1; 10];
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let commission = UNIT_BALANCE * 1;
		let exit_fee = UNIT_BALANCE * 1;
		let public_fund_allowed = true;
		let trading_account = AccountId32::new([2; 32]);
		let market_maker = AccountId32::new([1; 32]);
		register_test_trading_pair();
		mint_base_quote_asset_for_user(market_maker.clone());
		assert_noop!(
			LiqudityMining::register_pool(
				RuntimeOrigin::signed(market_maker.clone()),
				name,
				trading_pair,
				commission,
				exit_fee,
				public_fund_allowed,
				trading_account.clone()
			),
			pallet_ocex_lmp::pallet::Error::<Test>::ProxyAlreadyRegistered
		);
		let (_pool, share_id) = LiqudityMining::create_pool_account(&market_maker, trading_pair);
		// Check if Asset is registered or not
		assert!(!Assets::asset_exists(share_id)); //Verify this with @gautham
	})
}
use frame_support::traits::{
	fungible::Mutate,
	fungibles::{ Mutate as MutateNonNative},
};

use pallet_ocex_lmp::pallet::PriceOracle;
use sp_runtime::{traits::One, ArithmeticError::Underflow};
#[test]
fn test_add_liquidity_happy_path() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		// Set snapshot flag
		//<SnapshotFlag<Test>>::put(None);
		// Allowlist Token
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), base_asset));
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), quote_asset));
		// Put average price in OCEX Pallet
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let mut price = Decimal::from_u128(UNIT_BALANCE * 5).unwrap();
		price.div_assign(Decimal::from(UNIT_BALANCE));
		let tick = Decimal::from_u128(UNIT_BALANCE * 1).unwrap();
		let market_maker = AccountId32::new([2; 32]);
		let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
		let mut map = BTreeMap::new();
		map.insert((base_asset, quote_asset), (price, tick));
		<PriceOracle<Test>>::set(map);
		// Cretae Base and Quote Asset;
		mint_base_quote_asset_for_user(user_who_wants_to_add_liq.clone());
		assert_ok!(LiqudityMining::add_liquidity(
			RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
			trading_pair,
			market_maker,
			UNIT_BALANCE * 6,
			UNIT_BALANCE * 40
		));
		// * Check user balance
		assert_eq!(Balances::free_balance(&user_who_wants_to_add_liq), UNIT_BALANCE * 94);
		// TODO: Check pool balance and pallet account balance
	})
}

#[test]
fn test_add_liquidity_error_public_fund_not_allowed() {
	new_test_ext().execute_with(|| {
		register_test_pool(false);
		let market_maker = AccountId32::new([2; 32]);
		let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		assert_noop!(
			LiqudityMining::add_liquidity(
				RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
				trading_pair,
				market_maker,
				UNIT_BALANCE * 6,
				UNIT_BALANCE * 40
			),
			crate::pallet::Error::<Test>::PublicDepositsNotAllowed
		);
	})
}

#[test]
fn test_add_liquidity_error_price_not_found() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		let market_maker = AccountId32::new([2; 32]);
		let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		assert_noop!(
			LiqudityMining::add_liquidity(
				RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
				trading_pair,
				market_maker,
				UNIT_BALANCE * 6,
				UNIT_BALANCE * 40
			),
			crate::pallet::Error::<Test>::PriceNotAvailable
		);
	})
}

#[test]
fn test_add_liquidity_error_not_enough_quote_amount() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		// Set snapshot flag
		//<SnapshotFlag<Test>>::put(None);
		// Allowlist Token
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), base_asset));
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), quote_asset));
		// Put average price in OCEX Pallet
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let mut price = Decimal::from_u128(UNIT_BALANCE * 5).unwrap();
		price.div_assign(Decimal::from(UNIT_BALANCE));
		let tick = Decimal::from_u128(UNIT_BALANCE * 1).unwrap();
		let market_maker = AccountId32::new([2; 32]);
		let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
		let mut map = BTreeMap::new();
		map.insert((base_asset, quote_asset), (price, tick));
		<PriceOracle<Test>>::set(map);
		// Cretae Base and Quote Asset;
		mint_base_quote_asset_for_user(user_who_wants_to_add_liq.clone());
		assert_noop!(
			LiqudityMining::add_liquidity(
				RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
				trading_pair,
				market_maker,
				UNIT_BALANCE * 6,
				UNIT_BALANCE * 10
			),
			crate::pallet::Error::<Test>::NotEnoughQuoteAmount
		);
	})
}

#[test]
fn test_add_liquidity_not_enough_token_to_trasnfer() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		// Set snapshot flag
		//<SnapshotFlag<Test>>::put(None);
		// Allowlist Token
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), base_asset));
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), quote_asset));
		// Put average price in OCEX Pallet
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let mut price = Decimal::from_u128(UNIT_BALANCE * 5).unwrap();
		price.div_assign(Decimal::from(UNIT_BALANCE));
		let tick = Decimal::from_u128(UNIT_BALANCE * 1).unwrap();
		let market_maker = AccountId32::new([2; 32]);
		let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
		let mut map = BTreeMap::new();
		map.insert((base_asset, quote_asset), (price, tick));
		<PriceOracle<Test>>::set(map);
		// Cretae Base and Quote Asset;
		mint_base_quote_asset_for_user(user_who_wants_to_add_liq.clone());
		assert_noop!(
			LiqudityMining::add_liquidity(
				RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
				trading_pair,
				market_maker,
				UNIT_BALANCE * 10000,
				UNIT_BALANCE * 40000000
			),
			Underflow
		);
	})
}

#[test]
fn test_remove_liquidity_happy_path_and_error() {
	new_test_ext().execute_with(|| {
		add_liquidity();
		let market_maker = AccountId32::new([2; 32]);
		let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		assert_ok!(LiqudityMining::remove_liquidity(
			RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
			trading_pair,
			market_maker.clone(),
			UNIT_BALANCE * 6
		));
		let (_pool, share_id) = LiqudityMining::create_pool_account(&market_maker, trading_pair);
		// * Check shares of user
		assert_eq!(Assets::balance(share_id, &user_who_wants_to_add_liq), 0);
		assert_noop!(
			LiqudityMining::remove_liquidity(
				RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
				trading_pair,
				market_maker.clone(),
				UNIT_BALANCE * 6
			),
			crate::pallet::Error::<Test>::TotalShareIssuanceIsZero
		);
	})
}

#[test]
fn test_force_close_pool_happy_path_and_error() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let market_maker = AccountId32::new([2; 32]);
		let base_freed = Decimal::from(2);
		let quote_freed = Decimal::from(3);
		assert_ok!(LiqudityMining::pool_force_close_success(
			trading_pair,
			&market_maker,
			base_freed,
			quote_freed
		));
		assert_ok!(LiqudityMining::force_close_pool(
			RuntimeOrigin::root(),
			trading_pair,
			market_maker.clone()
		));
		let config = <Pools<Test>>::get(trading_pair, market_maker.clone()).unwrap();
		assert_eq!(config.force_closed, true);
	})
}

#[test]
fn test_add_liquidity_success_happy_path() {
	new_test_ext().execute_with(|| {
		// Create Pool
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let market_maker = AccountId32::new([2; 32]);
		let lp = AccountId32::new([3; 32]);
		let share_issued: Decimal = Decimal::from(100);
		let price: Decimal = Decimal::from(5);
		let total_inventory_in_quote: Decimal = Decimal::from(40);
		register_test_pool(true);
		// Start new epoch
		LiqudityMining::new_epoch(1);
		assert_ok!(LiqudityMining::add_liquidity_success(
			trading_pair,
			&market_maker,
			&lp,
			share_issued,
			price,
			total_inventory_in_quote
		));
	})
}
#[test]
fn test_submit_scores_of_lps_happy_path() {
	new_test_ext().execute_with(|| {
		let market_maker = AccountId32::new([2; 32]);
		let mut score_map: BTreeMap<AccountId32, (u128, bool)> = BTreeMap::new();
		score_map.insert(market_maker.clone(), (100 * UNIT_BALANCE, true));
		let total_score = 100 * UNIT_BALANCE;
		let mut results: BTreeMap<
			(TradingPair, AccountId32, u16),
			(BTreeMap<AccountId32, (u128, bool)>, u128),
		> = BTreeMap::new();
		results.insert(
			(
				TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
				market_maker.clone(),
				1,
			),
			(score_map, total_score),
		);
		register_test_pool(true);
		assert_ok!(LiqudityMining::submit_scores_of_lps(RuntimeOrigin::none(), results));
	})
}

use orderbook_primitives::{TraderMetricsMap, TradingPairMetrics, TradingPairMetricsMap};
use sp_runtime::traits::AccountIdConversion;

#[test]
fn test_claim_rewards_by_lp_happy_path_and_error() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		add_lmp_config();
		update_lmp_score();
		let reward_account =
			<crate::mock::Test as pallet_ocex_lmp::Config>::LMPRewardsPalletId::get()
				.into_account_truncating();
		Balances::mint_into(&reward_account, 300 * UNIT_BALANCE).unwrap();
		let market_maker = AccountId32::new([2; 32]);
		let trader = AccountId32::new([1; 32]);
		let mut score_map: BTreeMap<AccountId32, (u128, bool)> = BTreeMap::new();
		score_map.insert(trader.clone(), (100 * UNIT_BALANCE, false));
		let total_score = 100 * UNIT_BALANCE;
		let mut results: BTreeMap<
			(TradingPair, AccountId32, u16),
			(BTreeMap<AccountId32, (u128, bool)>, u128),
		> = BTreeMap::new();
		results.insert(
			(
				TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
				market_maker.clone(),
				1,
			),
			(score_map, total_score),
		);
		assert_ok!(LiqudityMining::submit_scores_of_lps(RuntimeOrigin::none(), results));
		assert_ok!(LiqudityMining::claim_rewards_by_lp(
			RuntimeOrigin::signed(trader.clone()),
			TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
			market_maker.clone(),
			1
		));
		assert_noop!(
			LiqudityMining::claim_rewards_by_lp(
				RuntimeOrigin::signed(trader.clone()),
				TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
				market_maker.clone(),
				1
			),
			crate::pallet::Error::<Test>::AlreadyClaimed
		);
	})
}

#[test]
fn test_claim_rewards_by_mm_happy_path_and_error() {
	new_test_ext().execute_with(|| {
		register_test_pool(true);
		add_lmp_config();
		update_lmp_score();
		let reward_account =
			<crate::mock::Test as pallet_ocex_lmp::Config>::LMPRewardsPalletId::get()
				.into_account_truncating();
		Balances::mint_into(&reward_account, 300 * UNIT_BALANCE).unwrap();
		let market_maker = AccountId32::new([2; 32]);
		let trader = AccountId32::new([1; 32]);
		let mut score_map: BTreeMap<AccountId32, (u128, bool)> = BTreeMap::new();
		score_map.insert(trader.clone(), (100 * UNIT_BALANCE, false));
		let total_score = 100 * UNIT_BALANCE;
		let mut results: BTreeMap<
			(TradingPair, AccountId32, u16),
			(BTreeMap<AccountId32, (u128, bool)>, u128),
		> = BTreeMap::new();
		results.insert(
			(
				TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
				market_maker.clone(),
				1,
			),
			(score_map, total_score),
		);
		assert_ok!(LiqudityMining::submit_scores_of_lps(RuntimeOrigin::none(), results));
		assert_ok!(LiqudityMining::claim_rewards_by_mm(
			RuntimeOrigin::signed(market_maker.clone()),
			TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
			1
		));
		assert_noop!(
			LiqudityMining::claim_rewards_by_mm(
				RuntimeOrigin::signed(market_maker.clone()),
				TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
				1
			),
			crate::pallet::Error::<Test>::AlreadyClaimed
		);
	})
}

use crate::pallet::WithdrawalRequests;

#[test]
fn test_initiate_withdrawal() {
	new_test_ext().execute_with(|| {
		// Register pool
		register_test_pool(true);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let epoch = 1;
		let num_of_request = 1;
		let market_maker = AccountId32::new([2; 32]);
		let (pool, _share_id) = LiqudityMining::create_pool_account(&market_maker, trading_pair);
		let trader = AccountId32::new([1; 32]);
		let asset1 = 10 * UNIT_BALANCE;
		let asset2 = 10 * UNIT_BALANCE;
		let mut value = Vec::new();
		value.push((trader, asset1, asset2));
		<WithdrawalRequests<Test>>::insert(epoch, pool, value);
		// Remove liquidity
		assert_ok!(LiqudityMining::initiate_withdrawal(
			RuntimeOrigin::signed(market_maker),
			trading_pair,
			epoch,
			num_of_request
		));
	})
}

pub fn update_lmp_score() {
	let total_score = Decimal::from(1000);
	let total_fee_paid = Decimal::from(1000);
	let trading_pair_metrics: TradingPairMetrics = (total_score, total_fee_paid);
	let trader = AccountId32::new([1; 32]);
	let trader_score = Decimal::from(100);
	let trader_fee_paid = Decimal::from(100);
	let mut trader_metrics: TraderMetricsMap<AccountId32> = BTreeMap::new();
	trader_metrics.insert(trader.clone(), (trader_score, trader_fee_paid));
	let mut trading_pair_metrics_map: TradingPairMetricsMap<AccountId32> = BTreeMap::new();
	trading_pair_metrics_map.insert(
		TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
		(trader_metrics, trading_pair_metrics),
	);
	assert_ok!(OCEX::update_lmp_scores(&trading_pair_metrics_map));
}

pub fn add_lmp_config() {
	let total_liquidity_mining_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
	let total_trading_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
	// Register trading pair
	let mut market_weightage = BTreeMap::new();
	market_weightage.insert(trading_pair.clone(), UNIT_BALANCE);
	let market_weightage: Option<BTreeMap<TradingPair, u128>> = Some(market_weightage);
	let mut min_fees_paid = BTreeMap::new();
	min_fees_paid.insert(trading_pair.clone(), UNIT_BALANCE);
	let min_fees_paid: Option<BTreeMap<TradingPair, u128>> = Some(min_fees_paid);
	let mut min_maker_volume = BTreeMap::new();
	min_maker_volume.insert(trading_pair, UNIT_BALANCE);
	let min_maker_volume: Option<BTreeMap<TradingPair, u128>> = Some(min_maker_volume);
	let max_accounts_rewarded: Option<u16> = Some(10);
	let claim_safety_period: Option<u32> = Some(0);
	assert_ok!(OCEX::set_lmp_epoch_config(
		RuntimeOrigin::root(),
		total_liquidity_mining_rewards,
		total_trading_rewards,
		market_weightage,
		min_fees_paid,
		min_maker_volume,
		max_accounts_rewarded,
		claim_safety_period
	));
	OCEX::start_new_epoch();
	OCEX::start_new_epoch();
}

use orderbook_primitives::traits::LiquidityMiningCrowdSourcePallet;

fn add_liquidity() {
	register_test_pool(true);
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), base_asset));
	assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), quote_asset));
	let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
	let mut price = Decimal::from_u128(UNIT_BALANCE * 5).unwrap();
	price.div_assign(Decimal::from(UNIT_BALANCE));
	let tick = Decimal::from_u128(UNIT_BALANCE * 1).unwrap();
	let market_maker = AccountId32::new([2; 32]);
	let user_who_wants_to_add_liq = AccountId32::new([3; 32]);
	let mut map = BTreeMap::new();
	map.insert((base_asset, quote_asset), (price, tick));
	<PriceOracle<Test>>::set(map);
	// Cretae Base and Quote Asset;
	mint_base_quote_asset_for_user(user_who_wants_to_add_liq.clone());
	assert_ok!(LiqudityMining::add_liquidity(
		RuntimeOrigin::signed(user_who_wants_to_add_liq.clone()),
		trading_pair,
		market_maker.clone(),
		UNIT_BALANCE * 6,
		UNIT_BALANCE * 40
	));
	let share_issued = Decimal::from(6);
	let price = Decimal::from(5);
	let total_inventory_in_quote = Decimal::from(40);
	assert_ok!(LiqudityMining::add_liquidity_success(
		trading_pair,
		&market_maker,
		&user_who_wants_to_add_liq,
		share_issued,
		price,
		total_inventory_in_quote
	));
}

fn mint_base_quote_asset_for_user(user: AccountId32) {
	let quote_asset = AssetId::Asset(1);
	Balances::mint_into(&user, UNIT_BALANCE * 100).unwrap();
	Assets::create(
		RuntimeOrigin::signed(user.clone()),
		parity_scale_codec::Compact(quote_asset.asset_id().unwrap()),
		AccountId32::new([1; 32]),
		One::one(),
	).unwrap();
	assert_ok!(Assets::mint_into(quote_asset.asset_id().unwrap(), &user, UNIT_BALANCE * 100));
}

fn register_test_pool(public_fund_allowed: bool) {
	let name = [1; 10];
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
	let commission = UNIT_BALANCE * 1;
	let exit_fee = UNIT_BALANCE * 1;
	let trading_account = AccountId32::new([1; 32]);
	let market_maker = AccountId32::new([2; 32]);
	register_test_trading_pair();
	mint_base_quote_asset_for_user(market_maker.clone());
	assert_ok!(LiqudityMining::register_pool(
		RuntimeOrigin::signed(market_maker.clone()),
		name,
		trading_pair,
		commission,
		exit_fee,
		public_fund_allowed,
		trading_account.clone()
	));
}

fn register_test_trading_pair() {
	let base = AssetId::Polkadex;
	let quote = AssetId::Asset(1);
	let min_order_price: u128 = UNIT_BALANCE * 2;
	let max_order_price: u128 = UNIT_BALANCE * 10;
	let min_order_qty: u128 = UNIT_BALANCE * 2;
	let max_order_qty: u128 = UNIT_BALANCE * 10;
	let price_tick_size: u128 = UNIT_BALANCE;
	let qty_step_size: u128 = UNIT_BALANCE;
	assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
	assert_ok!(OCEX::register_trading_pair(
		RuntimeOrigin::root(),
		base,
		quote,
		min_order_price,
		max_order_price,
		min_order_qty,
		max_order_qty,
		price_tick_size,
		qty_step_size
	));
}
