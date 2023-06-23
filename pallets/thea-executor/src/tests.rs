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
	mock::{new_test_ext, Assets, Test, *},
	PendingWithdrawals, WithdrawalFees,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungible::Mutate as FungibleMutate, fungibles::Mutate as FungiblesMutate},
};
use frame_system::EventRecord;
use parity_scale_codec::Encode;
use sp_runtime::traits::BadOrigin;
use thea_primitives::types::{Deposit, Withdraw};

fn assert_last_event<T: crate::Config>(generic_event: <T as crate::Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

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
		Balances::set_balance(&admin, 1_000_000_000_000_000_000);
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		// Set balance for User
		Balances::set_balance(&user, 1_000_000_000_000_000_000);
		assert_ok!(Assets::mint_into(asset_id, &user, 1_000_000_000_000_000_000));
		// Set withdrawal Fee
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), 1, 0));
		assert_ok!(TheaExecutor::withdraw(
			RuntimeOrigin::signed(user),
			asset_id,
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
		Balances::set_balance(&admin, 1_000_000_000_000_000_000);
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
		Balances::set_balance(&admin, 1_000_000_000_000_000_000);
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		Balances::set_balance(&recipient, 1_000_000_000_000_000_000);
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
		Balances::set_balance(&admin, 1_000_000_000_000_000_000);
		assert_ok!(Assets::create(
			RuntimeOrigin::signed(admin),
			parity_scale_codec::Compact(asset_id),
			admin,
			1u128
		));
		Balances::set_balance(&recipient, 1_000_000_000_000_000_000);
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

#[test]
fn test_set_withdrawal_fee_full() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_noop!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::none(), 1, 1), BadOrigin);
		assert!(<WithdrawalFees<Test>>::get(1).is_none());
		assert_noop!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::signed(1), 1, 1), BadOrigin);
		assert!(<WithdrawalFees<Test>>::get(1).is_none());
		// proper origin
		// max inputs
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), u8::MAX, u128::MAX));
		assert_eq!(<WithdrawalFees<Test>>::get(u8::MAX).unwrap(), u128::MAX);
		// half max inputs
		assert_ok!(TheaExecutor::set_withdrawal_fee(
			RuntimeOrigin::root(),
			u8::MAX / 2,
			u128::MAX / 2
		));
		// min inputs
		System::set_block_number(1);
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), 0, 0));
		assert_last_event::<Test>(crate::Event::<Test>::WithdrawalFeeSet(0, 0).into());
	})
}
