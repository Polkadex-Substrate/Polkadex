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

use crate::{
	mock::{new_test_ext, Assets, RuntimeOrigin as Origin, Test, *},
	PendingWithdrawals,
};

use asset_handler::pallet::Error;
use frame_support::{assert_err, assert_noop, assert_ok, traits::fungibles::Mutate};
use parity_scale_codec::Encode;
use sp_core::{H160, H256};
use sp_runtime::{traits::ConstU32, BoundedVec, SaturatedConversion, TokenError};
use thea_primitives::types::{Deposit, Withdraw};
use xcm::{
	latest::{AssetId, Fungibility, Junction, Junctions, MultiAsset, MultiLocation, NetworkId},
	prelude::X1,
};

#[test]
fn test_withdraw_returns_ok() {
	new_test_ext().execute_with(|| {
		// Insert authority
		let beneficiary: [u8; 1001] = [1; 1001];
		assert_noop!(
			TheaExecutor::withdraw(
				RuntimeOrigin::signed(1),
				1u128,
				1000u128,
				beneficiary.to_vec(),
				false,
				1
			),
			crate::Error::<Test>::BeneficiaryTooLong
		);
	})
}

#[test]
fn test_transfer_native_asset() {
	new_test_ext().execute_with(|| {
		// Create Asset
		let asset_id = 1000u128;
		let admin = 1u64;
		let user = 2u64;
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			admin,
			1_000_000_000_000_000_000,
			0
		));
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		// Set balance for User
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			user,
			1_000_000_000_000_000_000,
			0
		));
		assert_ok!(Assets::mint_into(asset_id, &user, 1_000_000_000_000_000_000));
		// Set withdrawal Fee
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), 1, 0));
		assert_ok!(TheaExecutor::withdraw(
			RuntimeOrigin::signed(1),
			asset_id.clone(),
			10_000_000_000_000u128,
			vec![1; 32],
			false,
			1
		));
		// Verify
		let pending_withdrawal = <PendingWithdrawals<Test>>::get(1);
		let approved_withdraw = Withdraw {
			id: Vec::from([
				179, 96, 16, 235, 40, 92, 21, 74, 140, 214, 48, 132, 172, 190, 126, 172, 12, 77,
				98, 90, 180, 225, 167, 110, 98, 74, 135, 152, 203, 99, 73, 123,
			]),
			asset_id,
			amount: 10_000_000_000_000u128,
			destination: vec![1; 32],
			is_blocked: false,
			extra: vec![],
		};
		assert_eq!(pending_withdrawal.to_vec().pop().unwrap(), approved_withdraw);
	})
}

#[test]
fn test_deposit_with_valid_args_returns_ok() {
	new_test_ext().execute_with(|| {
		let asset_id = 1000u128;
		let admin = 1u64;
		let recipient = 2u64;
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			admin,
			1_000_000_000_000_000_000,
			0
		));
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		let deposit = Deposit {
			id: Vec::new(),
			recipient,
			asset_id,
			amount: 1_000_000_000_000_000_000u128,
			extra: vec![],
		};
		assert_ok!(TheaExecutor::do_deposit(1, vec![deposit].encode()));
	})
}

#[test]
fn test_claim_deposit_returns_ok() {
	new_test_ext().execute_with(|| {
		let asset_id = 2000u128;
		let admin = 1u64;
		let recipient = 2u64;
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			admin,
			1_000_000_000_000_000_000,
			0
		));
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			recipient,
			1_000_000_000_000_000_000,
			0
		));
		let deposit = Deposit {
			id: Vec::new(),
			recipient,
			asset_id,
			amount: 1_000_000_000_000_000_000u128,
			extra: vec![],
		};
		assert_ok!(TheaExecutor::do_deposit(1, vec![deposit].encode()));
		assert_ok!(TheaExecutor::claim_deposit(RuntimeOrigin::signed(recipient), 1));
	})
}

#[test]
fn test_claim_deposit_returns_asset_not_registered() {
	new_test_ext().execute_with(|| {
		let asset_id = 2000u128;
		let admin = 1u64;
		let recipient = 2u64;
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			admin,
			1_000_000_000_000_000_000,
			0
		));
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		assert_ok!(Balances::set_balance(
			RuntimeOrigin::root(),
			recipient,
			1_000_000_000_000_000_000,
			0
		));
		let deposit = Deposit {
			id: Vec::new(),
			recipient,
			asset_id,
			amount: 1_000_000_000_000_000_000u128,
			extra: vec![],
		};
		assert_ok!(TheaExecutor::do_deposit(1, vec![deposit].encode()));
		assert_noop!(
			TheaExecutor::claim_deposit(RuntimeOrigin::signed(recipient), 1),
			crate::Error::<Test>::AssetNotRegistered
		);
	})
}
