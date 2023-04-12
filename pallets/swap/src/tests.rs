// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::*;
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use support::AMM;

const MINIMUM_LIQUIDITY: u128 = 1_000;

#[test]
fn create_pool_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 2_000);
		assert_eq!(Assets::total_issuance(SAMPLE_LP_TOKEN), 1_414);
		// should be issuance minus the min liq locked
		assert_eq!(Assets::balance(SAMPLE_LP_TOKEN, BOB), 414);
		assert_eq!(Swap::get_pool_by_lp_asset(SAMPLE_LP_TOKEN).is_some(), true);
		assert_eq!(Swap::get_pool_by_asset_pair((DOT, SDOT)).is_some(), true);
		assert_eq!(Swap::get_pool_by_asset_pair((SDOT, DOT)).is_some(), true);
	})
}

#[test]
fn double_liquidity_correct_liq_ratio_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, KSM),                      /* Currency pool, in which liquidity
			                                  * will be added */
			(15_000_000_000_000, 50_000_000_000_000_000), // Liquidity amounts to be added in pool
			FRANK,                                        // LPToken receiver
			SAMPLE_LP_TOKEN,                              /* Liquidity pool share representative
			                                               * token */
		));

		// total liquidity after pool created
		let total_liquidity_tokens = Assets::total_issuance(SAMPLE_LP_TOKEN);

		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(FRANK).into(), // Origin
			(DOT, KSM),                      /* Currency pool, in which liquidity
			                                  * will be added */
			(15_000_000_000_000, 50_000_000_000_000_000), // Liquidity amounts to be added in pool
			(15_000_000_000_000, 50_000_000_000_000_000), /* specifying its worst case ratio
			                                               * when pool already */
		));

		let total_liquidity_tokens_after_double = Assets::total_issuance(SAMPLE_LP_TOKEN);
		let liquidity_received = total_liquidity_tokens_after_double - total_liquidity_tokens;

		// received liquidity should be half of total liquidity
		assert_eq!(liquidity_received as f64 / total_liquidity_tokens_after_double as f64, 0.5);
	})
}

#[test]
fn add_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));
		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			(5, 5),                          // specifying its worst case ratio when pool already
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 4_000);
	})
}

#[test]
fn add_more_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(3_000, 4_000),                  // Liquidity amounts to be added in pool
			(5, 5),                          /* specifying its worst case ratio when pool
			                                  * already exists */
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 6_000);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 3_000);
	})
}

#[test]
fn add_more_liquidity_should_not_work_if_minimum_base_amount_is_higher() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		assert_noop!(
			Swap::add_liquidity(
				RawOrigin::Signed(ALICE).into(), // Origin
				(DOT, SDOT),                     // Currency pool, in which liquidity will be added
				(3_000, 4_000),                  // Liquidity amounts to be added in pool
				(5_500, 5_00)                    /* specifying its worst case ratio when pool
				                                  * already */
			),
			Error::<Test>::NotAnIdealPrice // Not an ideal price ratio
		);
	})
}

#[test]
fn add_more_liquidity_with_low_balance_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(3_000, 4_000),                  // Liquidity amounts to be added in pool
			(1, 1),                          // specifying its worst case ratio when pool already
		));

		assert_noop!(
			Swap::add_liquidity(
				RawOrigin::Signed(ALICE).into(), // Origin
				(DOT, SDOT),                     // Currency pool, in which liquidity will be added
				(5000_000_000, 6000_000_000),    // Liquidity amounts to be added in pool
				(5, 5),                          /* specifying its worst case ratio when pool
				                                  * already */
			),
			pallet_assets::Error::<Test>::BalanceLow
		);
	})
}

#[test]
fn add_liquidity_by_another_user_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(3_000, 4_000),                  // Liquidity amounts to be added in pool
			(5, 5),                          // specifying its worst case ratio when pool already
		));

		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(BOB).into(), // Origin
			(DOT, SDOT),                   // Currency pool, in which liquidity will be added
			(500, 1_000),                  // Liquidity amounts to be added in pool
			(5, 5),                        // specifying its worst case ratio when pool already
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 7_000);
	})
}

#[test]
fn cannot_create_pool_twice() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		assert_noop!(
			Swap::create_pool(
				RawOrigin::Signed(ALICE).into(), // Origin
				(DOT, SDOT),                     // Currency pool, in which liquidity will be added
				(1_000, 2_000),                  // Liquidity amounts to be added in pool
				ALICE,                           // LPToken receiver
				SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
			),
			Error::<Test>::PoolAlreadyExists, // Pool already not exist
		);
	})
}

#[test]
fn remove_liquidity_whole_share_should_work() {
	new_test_ext().execute_with(|| {
		// A pool with a single LP provider
		// who deposit tokens and withdraws their whole share
		// (most simple case)

		let _ = Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 9_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		);

		assert_ok!(Swap::remove_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be removed
			3_000 - MINIMUM_LIQUIDITY        // liquidity to be removed from user's liquidity
		));
	})
}

#[test]
fn remove_liquidity_only_portion_should_work() {
	new_test_ext().execute_with(|| {
		// A pool with a single LP provider who
		// deposit tokens and withdraws
		// a portion of their total shares (simple case)

		let _ = Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 9_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		);

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 9_000);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 1_000);

		assert_ok!(Swap::remove_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be removed
			1_500                            // Liquidity to be removed from user's liquidity
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 4_500);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 500);
	})
}

#[test]
fn remove_liquidity_user_more_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_500),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));
		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_500, 3_000),                  // Liquidity amounts to be added in pool
			(5, 5),                          // specifying its worst case ratio when pool already
		));

		assert_ok!(Swap::remove_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be removed
			1_500                            // Liquidity to be removed from user's liquidity
		));
	})
}

#[test]
fn remove_liquidity_when_pool_does_not_exist_should_not_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Swap::remove_liquidity(RawOrigin::Signed(ALICE).into(), (DOT, SDOT), 15),
			Error::<Test>::PoolDoesNotExist
		);
	})
}

#[test]
fn remove_liquidity_with_more_liquidity_should_not_work() {
	new_test_ext().execute_with(|| {
		// A pool with a single LP provider
		// who deposit tokens and withdraws their whole share
		// (most simple case)

		let _ = Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 9_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		);

		assert_noop!(
			Swap::remove_liquidity(RawOrigin::Signed(ALICE).into(), (DOT, SDOT), 3_0000),
			Error::<Test>::InsufficientLiquidity
		);
	})
}

#[test]
fn swap_should_work_base_to_quote() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(100_000_000, 100_000_000),      // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		// SDOT is base_asset 1001
		// DOT is quote_asset 101

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 100_000_000); // SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000_000); // DOT

		let path = vec![DOT, SDOT];

		let amount_in = 1_000;

		let amounts_out = Swap::get_amounts_out(amount_in, path).unwrap();

		// check balances before swap
		assert_eq!(Assets::balance(DOT, trader), 1_000_000_000);
		assert_eq!(Assets::balance(SDOT, trader), 1_000_000_000);

		assert_ok!(Swap::swap(&trader, (DOT, SDOT), amounts_out[0]));

		assert_eq!(
			Assets::balance(DOT, trader),
			1_000_000_000 - amount_in // 999_999_000
		);

		assert_eq!(
			Assets::balance(SDOT, trader),
			1_000_000_000 + amounts_out[1] // 1_000_000_996
		);
	})
}

#[test]
fn swap_should_work_different_ratio_base_to_quote() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(100_000_000, 50_000_000),       // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		// SDOT is base_asset 1001
		// DOT is quote_asset 101

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 50_000_000); // SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000_000); // DOT

		let path = vec![DOT, SDOT];

		let amount_in = 1_000;

		let amounts_out = Swap::get_amounts_out(amount_in, path).unwrap();

		// check balances before swap
		assert_eq!(Assets::balance(DOT, trader), 1_000_000_000);
		assert_eq!(Assets::balance(SDOT, trader), 1_000_000_000);

		assert_ok!(Swap::swap(&trader, (DOT, SDOT), amounts_out[0],));

		assert_eq!(
			Assets::balance(DOT, trader),
			1_000_000_000 - amount_in // 999_999_000
		);

		assert_eq!(
			Assets::balance(SDOT, trader),
			1_000_000_000 + amounts_out[1] // 1_000_000_996
		);
	})
}

#[test]
fn swap_should_work_quote_to_base() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(SDOT, DOT),                     // Currency pool, in which liquidity will be added
			(50_000_000, 100_000_000),       // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		// SDOT is base_asset 1001
		// DOT is quote_asset 101

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 50_000_000); // SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000_000); // DOT

		let path = vec![DOT, SDOT];

		let amount_in = 1_000;

		let amounts_out = Swap::get_amounts_out(amount_in, path).unwrap();

		// check balances before swap
		assert_eq!(Assets::balance(DOT, trader), 1_000_000_000);
		assert_eq!(Assets::balance(SDOT, trader), 1_000_000_000);

		assert_ok!(Swap::swap(&trader, (DOT, SDOT), amounts_out[0],));

		assert_eq!(
			Assets::balance(DOT, trader),
			1_000_000_000 - amount_in // 999_999_000
		);

		assert_eq!(
			Assets::balance(SDOT, trader),
			1_000_000_000 + amounts_out[1] // 1_000_000_996
		);
	})
}

#[test]
fn trade_should_work_base_to_quote_flipped_currencies_on_pool_creation() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(SDOT, DOT),                     // Currency pool, in which liquidity will be added
			(100_000_000, 100_000_000),      // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		// SDOT is base_asset 1001
		// DOT is quote_asset 101

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 100_000_000); // SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000_000); // DOT

		// calculate amount out
		assert_ok!(Swap::swap(&trader, (DOT, SDOT), 1_000));

		assert_eq!(
			Assets::balance(SDOT, trader),
			1_000_000_000 + 996 // 1_000_000_996
		);

		// pools values should be updated - we should have less SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 99_999_004);

		// pools values should be updated - we should have more DOT in the pool
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_001_000);
	})
}

#[test]
fn trade_should_work_quote_to_base() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(100_000_000, 100_000_000),      // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		// SDOT is base_asset 1001
		// DOT is quote_asset 101

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 100_000_000); // SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000_000); // DOT

		// calculate amount out
		// trade base for quote
		assert_ok!(Swap::swap(&trader, (DOT, SDOT), 1_000));

		assert_eq!(
			Assets::balance(SDOT, trader),
			1_000_000_000 + 996 // 1_000_000_996
		);

		// we should have more DOT in the pool since were trading it for DOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_001_000);

		// we should have less SDOT since we traded it for SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 99_999_004);
	})
}

#[test]
fn trade_should_not_work_if_insufficient_amount_in() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(100_000, 100_000),              // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		// create pool and add liquidity
		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(CHARLIE).into(), // Origin
			(DOT, SDOT),                       // Currency pool, in which liquidity will be added
			(100_000, 100_000),                // Liquidity amounts to be added in pool
			(99_999, 99_999),                  // specifying its worst case ratio when pool already
		));

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 200_000); // SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 200_000); // DOT

		// amount out is less than minimum_amount_out
		assert_noop!(Swap::swap(&trader, (DOT, SDOT), 332), Error::<Test>::InsufficientAmountIn);
	})
}

#[test]
fn trade_should_work_flipped_currencies() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(100_000, 50_000),               // Liquidity amounts to be added in pool
			CHARLIE,                         // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		// check that pool was funded correctly
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000); // DOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 50_000); // SDOT

		// calculate amount out
		assert_ok!(Swap::swap(&trader, (DOT, SDOT), 500));

		assert_eq!(
			Assets::balance(SDOT, trader),
			1_000_000_000 + 247 //
		);

		// pools values should be updated - we should have less DOT in the pool
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().quote_amount, 100_000 + 500);

		// pools values should be updated - we should have more SDOT
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 50_000 - 247);
	})
}

#[test]
fn trade_should_not_work_if_amount_in_is_zero() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// create pool and add liquidity
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 1_000),                  // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		// fail if amount_in is zero
		assert_noop!(Swap::swap(&trader, (DOT, SDOT), 0), Error::<Test>::InsufficientAmountIn);
	})
}

#[test]
fn trade_should_not_work_if_pool_does_not_exist() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		// try to trade in pool with no liquidity
		assert_noop!(Swap::swap(&trader, (DOT, SDOT), 10), Error::<Test>::PoolDoesNotExist);
	})
}

#[test]
fn amount_out_should_work() {
	new_test_ext().execute_with(|| {
		let amount_in = 1_000;
		let supply_in = 100_000_000;
		let supply_out = 100_000_000;

		let amount_out = Swap::get_amount_out(amount_in, supply_in, supply_out).unwrap();

		// actual value == 996.9900600091017
		// TODO: assumes we round down to int
		assert_eq!(amount_out, 996);
	})
}

#[test]
fn amounts_out_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(1_000, 2_000),                  // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(KSM, DOT),                      // Currency pool, in which liquidity will be added
			(1_000, 1_000),                  // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN_2,               // Liquidity pool share representative token
		));

		let path = vec![SDOT, DOT, KSM];

		let amount_in = 1_000;

		let amounts_out = Swap::get_amounts_out(amount_in, path).unwrap();

		assert_eq!(amounts_out, [1000, 332, 248]);
	})
}

#[test]
fn long_route_amounts_in_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(10_000, 20_000),                // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(KSM, DOT),                      // Currency pool, in which liquidity will be added
			(10_000, 10_000),                // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN_2,               // Liquidity pool share representative token
		));

		let path = vec![SDOT, DOT, KSM];

		let amount_out = 1_000;

		let amounts_in = Swap::get_amounts_in(amount_out, path).unwrap();

		assert_eq!(amounts_in, [2521, 1116, 1000]);
	})
}

#[test]
fn short_route_amounts_in_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(10_000_000, 10_000_000),        // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		let path = vec![DOT, SDOT];

		let amount_out = 1_000;

		let amounts_in = Swap::get_amounts_in(amount_out, path).unwrap();

		assert_eq!(amounts_in, [1005, 1000]);
	})
}

#[test]
fn amount_in_should_work() {
	new_test_ext().execute_with(|| {
		let amount_out = 1_000;
		let supply_in = 100_000_000;
		let supply_out = 100_000_000;

		let amount_in = Swap::get_amount_in(amount_out, supply_in, supply_out).unwrap();
		// p = 1 - fee_percent
		// x * y = ( x + p * dx) ( y - dy)
		//
		// actual value == round_up(1002.5162908218259) + 1
		assert_eq!(amount_in, 1005)
	})
}

#[test]
fn amount_in_uneven_should_work() {
	new_test_ext().execute_with(|| {
		let amount_out = 1_000;
		let supply_in = 100_000_000;
		let supply_out = 1_344_312_043;

		let amount_in = Swap::get_amount_in(amount_out, supply_in, supply_out).unwrap();

		assert_eq!(amount_in, 76);
	})
}

#[test]
fn supply_out_should_larger_than_amount_out() {
	// Test case for Panic when amount_out >= supply_out
	new_test_ext().execute_with(|| {
		let amount_out = 100_00;
		let supply_in = 100_000;
		let supply_out = 100_00;

		assert_noop!(
			Swap::get_amount_in(amount_out, supply_in, supply_out),
			Error::<Test>::InsufficientSupplyOut
		);
	})
}

#[test]
fn amount_out_and_in_should_work() {
	new_test_ext().execute_with(|| {
		let amount_out = 1_000;
		let supply_in = 100_000_000;
		let supply_out = 100_000_000;

		let amount_in = Swap::get_amount_in(amount_out, supply_in, supply_out).unwrap();

		// actual: 1002.5162908248136
		assert_eq!(amount_in, 1005);

		let amount_out = Swap::get_amount_out(amount_in, supply_in, supply_out).unwrap();

		// actual: 1000.0834982275963
		assert_eq!(amount_out, 1000);
	})
}

#[test]
fn update_oracle_should_work() {
	new_test_ext().execute_with(|| {
		let trader = EVE;

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(SDOT, DOT),                     // Currency pool, in which liquidity will be added
			(100_000, 100_000),              // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().block_timestamp_last, 0);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().price_0_cumulative_last, 0);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().price_1_cumulative_last, 0);

		run_to_block(2);

		assert_ok!(Swap::swap(&trader, (DOT, SDOT), 1_000));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().block_timestamp_last, 2);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().price_0_cumulative_last, 2_040136143738700978);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().price_1_cumulative_last, 1_960653465346534653);

		run_to_block(4);

		assert_ok!(Swap::swap(&trader, (DOT, SDOT), 1_000));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().block_timestamp_last, 4);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().price_0_cumulative_last, 4_120792162342213614);
		assert_eq!(Swap::pools(SDOT, DOT).unwrap().price_1_cumulative_last, 3_883124053581828770);
	})
}

#[test]
fn create_pool_large_amount_should_work() {
	/*
	With ample supplies
	Recheck values
	*/
	new_test_ext().execute_with(|| {
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			DOT.into(),
			ALICE,
			3_000_000_000_000_000_000_000,
		)
		.ok();
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			SDOT.into(),
			ALICE,
			2_000_000_000_000_000_000_000,
		)
		.ok();

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     /* Currency pool, in
			                                  * which liquidity will
			                                  * be added */
			(1_000_000_000_000_000_000, 2_000_000_000_000_000_000_000), /* Liquidity amounts to
			                                                             * be added in pool */
			ALICE,           // LPToken receiver
			SAMPLE_LP_TOKEN, // Liquidity pool share representative token
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 2_000_000_000_000_000_000_000);
		assert_eq!(Assets::total_issuance(SAMPLE_LP_TOKEN), 447_213_595_499_957_939_28);
		// should be issuance minus the min liq locked
		assert_eq!(Assets::balance(SAMPLE_LP_TOKEN, ALICE), 447_213_595_499_957_939_28);
	})
}

#[test]
fn create_pool_large_amount_from_an_account_without_sufficient_amount_of_tokens_should_not_panic() {
	/*
	With ample supplies for Alice and less for Bob :'(
	`create_pool` with Large amount panic for Bob
	Recheck values
	*/
	new_test_ext().execute_with(|| {
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			DOT.into(),
			ALICE,
			3_000_000_000_000_000_000_000,
		)
		.ok();
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			SDOT.into(),
			ALICE,
			2_000_000_000_000_000_000_000,
		)
		.ok();

		// Creating for BOB
		// This Panics!
		assert_noop!(
			Swap::create_pool(
				RawOrigin::Signed(ALICE).into(), // Origin
				(DOT, SDOT),                     /* Currency pool, in
				                                  * which liquidity
				                                  * will be added */
				(1_000_000_000_000_000_000, 2_000_000_000_000_000_000_000), /* Liquidity amounts
				                                                             * to be added in
				                                                             * pool */
				BOB,             // LPToken receiver
				SAMPLE_LP_TOKEN, // Liquidity pool share representative token
			),
			pallet_assets::Error::<Test>::BalanceLow
		);
	})
}

#[ignore]
#[test]
fn do_add_liquidity_exact_amounts_should_work() {
	/*
	substrate->frame->assets->src->functions.rs
	ensure!(f.best_effort || actual >= amount, Error::<T, I>::BalanceLow);   // Fails here
	replica of `add_liquidity_should_work` with larger values
	Loss of precision?
	*/
	new_test_ext().execute_with(|| {
		// Already deposited 100000000
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			DOT.into(),
			ALICE,
			999_999_999_999_900_000_000,
		)
		.ok();

		// Already deposited 100000000
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			SDOT.into(),
			ALICE,
			199_999_999_999_990_000_000_0,
		)
		.ok();

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     /* Currency pool, in
			                                  * which liquidity will
			                                  * be added */
			(1_000_000_000_000_000_000, 2_000_000_000_000_000_000_000), /* Liquidity amounts to
			                                                             * be added in pool */
			ALICE,           // LPToken receiver
			SAMPLE_LP_TOKEN, // Liquidity pool share representative token
		));
		assert_ok!(Swap::add_liquidity(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     /* Currency pool, in
			                                  * which liquidity will
			                                  * be added */
			(1_000_000_000_000_000_000, 2_000_000_000_000_000_000_000), /* Liquidity amounts to
			                                                             * be added in pool */
			(5, 5), // specifying its worst case ratio when pool already
		));

		assert_eq!(Swap::pools(SDOT, DOT).unwrap().base_amount, 4_000);
	})
}

#[test]
fn do_add_liquidity_large_amounts_should_work() {
	/*
	With ample supplies
	 */

	new_test_ext().execute_with(|| {
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			DOT.into(),
			ALICE,
			3_000_000_000_000_000_000_000,
		)
		.ok();
		Assets::mint(
			RawOrigin::Signed(ALICE).into(),
			SDOT.into(),
			ALICE,
			2_000_000_000_000_000_000_000,
		)
		.ok();

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(
				1_000_000_000_000_000_000_000, // Either base amount or quote amount
				2_000_000_000_000_000_000_000
			), // Liquidity amounts to be added in pool
			ALICE,                           // LPToken receiver
			SAMPLE_LP_TOKEN,                 // Liquidity pool share representative token
		));
	})
}

#[test]
fn update_protocol_fee_should_work() {
	new_test_ext().execute_with(|| {
		assert!(Swap::protocol_fee().is_zero());

		assert_ok!(Swap::update_protocol_fee(
			RuntimeOrigin::signed(ALICE),
			Ratio::from_percent(20)
		));

		assert_eq!(Swap::protocol_fee(), Ratio::from_percent(20));
	})
}

#[test]
fn update_protocol_fee_receiver_should_work() {
	new_test_ext().execute_with(|| {
		assert!(Swap::protolcol_fee_receiver().is_err());

		assert_ok!(Swap::update_protocol_fee_receiver(
			RuntimeOrigin::signed(ALICE),
			PROTOCOL_FEE_RECEIVER
		));

		assert_eq!(Swap::protolcol_fee_receiver().unwrap(), PROTOCOL_FEE_RECEIVER);
	})
}

#[test]
fn handling_fees_should_work() {
	new_test_ext().execute_with(|| {
		// Pool gets created and BOB should receive all of the LP tokens (minus the min amount)
		//
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(),    // Origin
			(DOT, SDOT),                        // Currency pool, in which liquidity will be added
			(100_000_000_000, 100_000_000_000), // Liquidity amounts to be added in pool
			BOB,                                // LPToken receiver
			SAMPLE_LP_TOKEN                     // Liquidity pool share representative token
		));

		assert_ok!(Swap::update_protocol_fee(
			RuntimeOrigin::signed(ALICE),
			Ratio::from_percent(20)
		));

		assert_ok!(Swap::update_protocol_fee_receiver(
			RuntimeOrigin::signed(ALICE),
			PROTOCOL_FEE_RECEIVER
		));

		// Another user makes a swap that should generate fees for the LP provider and the protocol
		assert_ok!(Swap::swap(&FRANK, (DOT, SDOT), 6_000_000));

		// we can check the total balance
		//
		// no extra fees should be minted becuase liquidty has not been added or removed
		//
		assert_eq!(Assets::total_issuance(SAMPLE_LP_TOKEN), 100_000_000_000);

		// bob should have all of the fees minus the min amount burned/locked
		assert_eq!(Assets::balance(SAMPLE_LP_TOKEN, BOB), 100_000_000_000 - MINIMUM_LIQUIDITY);

		// now we withdraw the fees and at this point we should mint tokens
		// for the protocol proportional to 1/5 of the total fees generated

		// we know that 18_000 fees should be collected and ~2_500 are for the protocol
		let total_fees_collected = 6_000_000.0 * 0.0025;
		let fees_to_be_collected_by_protocol = total_fees_collected * (0.20);
		assert_eq!(fees_to_be_collected_by_protocol, 3000.0);

		// expand the math to calculate exact amount of fees to dilute lp total supply
		let prop_of_total_fee = 1.0 / 5.0;
		let scalar = (1.0 / prop_of_total_fee) - 1.0;
		assert_eq!(scalar, 4.0);

		let total_lp_token_supply = 100_000_000_000.0;
		let old_root_k = (100_000_000_000f64 * 100_000_000_000f64).sqrt();
		let new_root_k = (99_994_015_358f64 * 100_006_000_000f64).sqrt();
		let root_k_growth = new_root_k - old_root_k;

		let numerator = total_lp_token_supply * root_k_growth;
		let denominator = new_root_k * scalar + old_root_k;
		let rewards_to_mint = numerator / denominator;

		assert_eq!(old_root_k, 100_000_000_000.0); // 100_000_000_000
		assert_eq!(new_root_k, 100000007499.46045); // 100_000_007_499

		assert_eq!(root_k_growth, 7499.46044921875); // 7499
		assert_eq!((numerator, denominator), (749946044921875.0, 500000029997.8418));
		assert_eq!(rewards_to_mint, 1499.8919998567042); // 1499

		assert_ok!(Swap::remove_liquidity(
			RawOrigin::Signed(PROTOCOL_FEE_RECEIVER).into(),
			(DOT, SDOT),
			1_285,
		));

		// PROTOCOL_FEE_RECEIVER should have slightly less then 3_000 total rewards
		// split between the two pools - the small difference is due to rounding errors
		assert_eq!(Assets::balance(DOT, PROTOCOL_FEE_RECEIVER), 1028);

		assert_eq!(Assets::balance(SDOT, PROTOCOL_FEE_RECEIVER), 1027);
	})
}

#[test]
fn swap_full_balance_should_work() {
	new_test_ext().execute_with(|| {
		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(),    // Origin
			(DOT, SDOT),                        // Currency pool, in which liquidity will be added
			(100_000_000_000, 100_000_000_000), // Liquidity amounts to be added in pool
			BOB,                                // LPToken receiver
			SAMPLE_LP_TOKEN                     // Liquidity pool share representative token
		));

		// user can swap all of their non native assets
		assert_ok!(Swap::swap(&FRANK, (DOT, SDOT), Assets::balance(DOT, FRANK)));

		assert_eq!(Assets::balance(DOT, FRANK), 0);
	})
}

#[test]
fn can_only_all_if_non_native_should_work() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Swap::create_pool(
				RawOrigin::Signed(ALICE).into(), // Origin
				(0, SDOT),                       // Currency pool, in which liquidity will be added
				(100000000, 100000000),          // Liquidity amounts to be added in pool
				BOB,                             // LPToken receiver
				SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
			),
			pallet_balances::Error::<Test>::ExistentialDeposit
		);

		assert_eq!(Balances::free_balance(BOB), 100_000_000_000_000);

		let all_dot = Assets::balance(DOT, BOB);
		let all_sdot = Assets::balance(SDOT, BOB);

		assert_ok!(Swap::create_pool(
			RawOrigin::Signed(ALICE).into(), // Origin
			(DOT, SDOT),                     // Currency pool, in which liquidity will be added
			(all_dot, all_sdot),             // Liquidity amounts to be added in pool
			BOB,                             // LPToken receiver
			SAMPLE_LP_TOKEN                  // Liquidity pool share representative token
		));

		assert_eq!(Assets::balance(DOT, BOB), 0);
		assert_eq!(Assets::balance(SDOT, BOB), 0);
	})
}

#[test]
fn quote_should_not_overflow() {
	assert_eq!(Swap::quote(u128::MAX, u128::MAX, u128::MAX), Ok(u128::MAX))
}

#[test]
fn glmr_add_liquidity_should_work() {
	new_test_ext().execute_with(|| {
		Assets::mint(RuntimeOrigin::signed(ALICE), GLMR.into(), ALICE, 1000000000000000000000000)
			.unwrap();
		Assets::mint(RuntimeOrigin::signed(ALICE), PARA.into(), ALICE, 200000000000000000000)
			.unwrap();

		Swap::create_pool(
			RawOrigin::Signed(ALICE).into(),
			(GLMR, PARA),
			(5978650946941927074614, 100290500000000000),
			ALICE,
			SAMPLE_LP_TOKEN,
		)
		.unwrap();

		assert_ok!(Swap::add_liquidity(
			RuntimeOrigin::signed(ALICE),
			(GLMR, PARA),
			(15000000000000000000000, 251621563685000000),
			(14925000000000000000000, 250363455866575000),
		));
	})
}
