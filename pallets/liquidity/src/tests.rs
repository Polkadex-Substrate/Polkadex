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

use crate::{mock::*, *};
use frame_support::{assert_noop, assert_ok};
use polkadex_primitives::{AccountId, AssetId};
use sp_runtime::{DispatchError::BadOrigin, SaturatedConversion};
pub const ALICE_ACCOUNT_RAW_ID: [u8; 32] = [0; 32];

fn get_alice_account() -> AccountId {
	AccountId::new(ALICE_ACCOUNT_RAW_ID)
}

fn get_account_generation_key() -> u32 {
	1
}

#[test]
fn register_pallet_account() {
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(RuntimeOrigin::root(), u32::MAX));
		assert_eq!(<RegisterGovernanceAccounts<Test>>::contains_key(u32::MAX), true);

		assert_ok!(Liquidity::register_account(RuntimeOrigin::root(), u32::MIN));
		assert_eq!(<RegisterGovernanceAccounts<Test>>::contains_key(u32::MIN), true);
	});
}

#[test]
fn try_to_register_pallet_account() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(RuntimeOrigin::root(), account_generation_key));
		assert_noop!(
			Liquidity::register_account(RuntimeOrigin::root(), account_generation_key),
			Error::<Test>::PalletAlreadyRegistered
		);
	});
}

#[test]
fn register_account_with_bad_origin() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::register_account(RuntimeOrigin::none(), account_generation_key),
			BadOrigin,
		);
		assert_noop!(
			Liquidity::register_account(
				RuntimeOrigin::signed(get_alice_account()),
				account_generation_key
			),
			BadOrigin,
		);
	});
}
#[test]
fn deposit() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(RuntimeOrigin::root(), account_generation_key));
		assert_ok!(Liquidity::deposit_to_orderbook(
			RuntimeOrigin::root(),
			AssetId::Polkadex,
			100_u128.saturated_into(),
			account_generation_key
		));
	});
}

#[test]
fn deposit_with_bad_origin() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::deposit_to_orderbook(
				RuntimeOrigin::none(),
				AssetId::Polkadex,
				100_u128.saturated_into(),
				account_generation_key
			),
			BadOrigin
		);
		assert_noop!(
			Liquidity::deposit_to_orderbook(
				RuntimeOrigin::signed(get_alice_account()),
				AssetId::Polkadex,
				100_u128.saturated_into(),
				account_generation_key
			),
			BadOrigin
		);
	});
}

#[test]
fn deposit_when_pallet_not_register() {
	let account_generation_key = get_account_generation_key();

	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::deposit_to_orderbook(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				100_u128.saturated_into(),
				account_generation_key
			),
			Error::<Test>::PalletAccountNotRegistered
		);
	});
}

#[test]
fn withdraw() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_ok!(Liquidity::register_account(RuntimeOrigin::root(), account_generation_key));
		assert_ok!(Liquidity::withdraw_from_orderbook(
			RuntimeOrigin::root(),
			AssetId::Polkadex,
			100_u128.saturated_into(),
			true,
			account_generation_key,
		));
	});
}

#[test]
fn withdraw_with_bad_origin() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::withdraw_from_orderbook(
				RuntimeOrigin::none(),
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true,
				account_generation_key
			),
			BadOrigin
		);
		assert_noop!(
			Liquidity::withdraw_from_orderbook(
				RuntimeOrigin::signed(get_alice_account()),
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true,
				account_generation_key
			),
			BadOrigin
		);
	});
}

#[test]
fn withdraw_when_pallet_not_register() {
	let account_generation_key = get_account_generation_key();
	new_test_ext().execute_with(|| {
		assert_noop!(
			Liquidity::withdraw_from_orderbook(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true,
				account_generation_key
			),
			Error::<Test>::PalletAccountNotRegistered
		);
	});
}
