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
	PendingWithdrawals, WithdrawalFees, *,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungible::Mutate as FungibleMutate, fungibles::Mutate as FungiblesMutate},
};
use frame_system::EventRecord;
use parity_scale_codec::Encode;
use sp_runtime::{
	traits::{AccountIdConversion, BadOrigin},
	SaturatedConversion,
};
use thea_primitives::types::{AssetMetadata, Deposit, Withdraw};
use xcm::{opaque::lts::Junctions, v3::MultiLocation, VersionedMultiLocation};

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
				1,
				false
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
			1,
			false
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
		assert_ok!(TheaExecutor::claim_deposit(RuntimeOrigin::signed(recipient), 1, recipient));
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
			TheaExecutor::claim_deposit(RuntimeOrigin::signed(3), 1, recipient),
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

#[test]
fn test_parachain_withdraw_full() {
	new_test_ext().execute_with(|| {
		// setup code
		let asset_id: <Test as Config>::AssetId = 100u128;
		let admin = 1u64;
		let network_id = 1;
		Balances::set_balance(&admin, 100_000_000_000_000_000_000u128.saturated_into());
		<Test as Config>::Currency::mint_into(
			&admin,
			100_000_000_000_000_000_000u128.saturated_into(),
		)
		.unwrap();
		<Test as Config>::Assets::create(
			RuntimeOrigin::signed(admin),
			asset_id.into(),
			admin,
			1u128.saturated_into(),
		)
		.unwrap();
		let pallet_acc = <Test as crate::Config>::TheaPalletId::get().into_account_truncating();
		Balances::set_balance(&pallet_acc, 100_000_000_000_000_000_000u128.saturated_into());
		<Test as Config>::Currency::mint_into(
			&pallet_acc,
			100_000_000_000_000_000_000u128.saturated_into(),
		)
		.unwrap();
		let account = 2u64;
		Balances::set_balance(&account, 100_000_000_000_000_000_000u128.saturated_into());
		<Test as Config>::Currency::mint_into(
			&account,
			100_000_000_000_000_000_000u128.saturated_into(),
		)
		.unwrap();
		Assets::mint_into(asset_id, &account, 100_000_000_000_000_000_000u128.saturated_into())
			.unwrap();
		<Test as Config>::Currency::mint_into(&account, 100_000_000_000_000u128.saturated_into())
			.unwrap();
		Balances::set_balance(&account, 100_000_000_000_000u128.saturated_into());
		let metadata = AssetMetadata::new(10).unwrap();
		<Metadata<Test>>::insert(100, metadata);
		<WithdrawalFees<Test>>::insert(network_id, 1_000);
		let multilocation = MultiLocation { parents: 1, interior: Junctions::Here };
		let beneficiary = Box::new(VersionedMultiLocation::V3(multilocation));
		// bad origins
		assert_noop!(
			TheaExecutor::parachain_withdraw(
				RuntimeOrigin::root(),
				u128::MAX,
				1_000_000_000,
				beneficiary.clone(),
				false,
				false
			),
			BadOrigin
		);
		assert_noop!(
			TheaExecutor::parachain_withdraw(
				RuntimeOrigin::none(),
				u128::MAX,
				1_000_000_000,
				beneficiary.clone(),
				false,
				false
			),
			BadOrigin
		);
		// asset not registered
		assert_noop!(
			TheaExecutor::parachain_withdraw(
				RuntimeOrigin::signed(account),
				u128::MAX,
				1_000_000_000,
				beneficiary.clone(),
				false,
				false
			),
			Error::<Test>::AssetNotRegistered
		);
		// funds unavailable
		assert_noop!(
			TheaExecutor::parachain_withdraw(
				RuntimeOrigin::signed(admin),
				asset_id,
				1_000_000_000,
				beneficiary.clone(),
				false,
				false
			),
			sp_runtime::TokenError::FundsUnavailable
		);
		// proper case
		assert_ok!(TheaExecutor::parachain_withdraw(
			RuntimeOrigin::signed(account),
			asset_id,
			1_000_000_000,
			beneficiary.clone(),
			false,
			false
		));
	})
}

#[test]
fn test_update_asset_metadata_full() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_noop!(
			TheaExecutor::update_asset_metadata(RuntimeOrigin::signed(1), 1, 1),
			BadOrigin
		);
		assert_noop!(
			TheaExecutor::update_asset_metadata(RuntimeOrigin::signed(u64::MAX), 1, 1),
			BadOrigin
		);
		assert_noop!(TheaExecutor::update_asset_metadata(RuntimeOrigin::none(), 1, 1), BadOrigin);
		// invalid decimal
		assert_noop!(
			TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), u128::MAX, u8::MIN),
			Error::<Test>::InvalidDecimal
		);
		// proper cases
		System::set_block_number(1);
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), 0, u8::MAX));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), u128::MAX, u8::MAX));
		let md = AssetMetadata::new(u8::MAX).unwrap();
		assert_last_event::<Test>(Event::<Test>::AssetMetadataSet(md).into());
	})
}

#[test]
fn test_resolve_deposit() {
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
		assert_ok!(TheaExecutor::execute_deposit(deposit, &recipient));
	})
}

#[test]
fn test_deposit_without_account() {
	new_test_ext().execute_with(|| {
		setup_pool();
		let asset_id = 1u128;
		let admin = 1u64;
		let recipient = 2u64;
		Balances::set_balance(&admin, 1_000_000_000_000_000_000);
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		Balances::set_balance(&TheaExecutor::thea_account(), 1_000_000_000_000_000_000);
		let deposit = Deposit {
			id: Vec::new(),
			recipient,
			asset_id,
			amount: 1_000_000_000_000_000u128,
			extra: vec![],
		};
		assert_ok!(TheaExecutor::execute_deposit(deposit, &recipient));
		assert_eq!(Balances::free_balance(&recipient), 50);
		assert_eq!(Assets::balance(asset_id, &recipient), 999_999_994_984_954u128);
		assert_eq!(Assets::balance(asset_id, &TheaExecutor::thea_account()), 0u128);
		assert_eq!(
			Balances::free_balance(&TheaExecutor::thea_account()),
			1_000_000_000_000_000_000
		);
	})
}

#[test]
fn test_do_withdrawal() {
	new_test_ext().execute_with(|| {
		setup_pool();
		let sender = 2u64;
		let asset_id = 1u128;
		// Set asset balance
		Balances::set_balance(&sender, 1_000_000_000_000_000_000);
		let _ = Assets::mint_into(asset_id, &sender, 1_000_000_000_000_000_000);
		// Set withdrawal Fee
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), 1, 100));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		assert_ok!(TheaExecutor::withdraw(
			RuntimeOrigin::signed(sender),
			asset_id,
			1_000_000_000_000_000u128,
			vec![1; 32],
			true,
			1,
			true
		));
		assert_eq!(Balances::free_balance(&sender), 1_000_000_000_000_000_000);
		assert_eq!(Assets::balance(asset_id, &sender), 999_000_000_000_000_000);
		assert_eq!(Balances::free_balance(&TheaExecutor::thea_account()), 1_000u128);
	})
}

#[test]
fn test_do_withdrawal_with_total_amount_consumed_returns_error() {
	new_test_ext().execute_with(|| {
		setup_pool();
		let sender = 2u64;
		let asset_id = 1u128;
		// Set asset balance
		let _ = Balances::set_balance(&sender, 1_000_000_000_000_000_000);
		assert_ok!(Assets::mint_into(asset_id, &sender, 100_300_903u128));
		// Set withdrawal Fee
		assert_ok!(TheaExecutor::set_withdrawal_fee(RuntimeOrigin::root(), 1, 100));
		assert_ok!(TheaExecutor::update_asset_metadata(RuntimeOrigin::root(), asset_id, 12));
		assert_noop!(
			TheaExecutor::withdraw(
				RuntimeOrigin::signed(sender),
				asset_id,
				1_000_000_000_000_000u128,
				vec![1; 32],
				true,
				1,
				true
			),
			sp_runtime::TokenError::FundsUnavailable
		);
	})
}

fn setup_pool() {
	let asset_id = 1u128;
	let admin = 1u64;
	Balances::set_balance(&admin, 2_000_000_000_000_000_000_000_000_000_000u128);
	assert_ok!(Assets::force_create(
		RuntimeOrigin::root(),
		parity_scale_codec::Compact(asset_id),
		admin,
		false,
		1u128
	));
	// Mint tokens
	Assets::mint_into(asset_id, &admin, 1_000_000_000_000_000_000_000_000_000u128).unwrap();
	// Create pool
	assert_ok!(AssetConversion::create_pool(
		RuntimeOrigin::signed(admin),
		polkadex_primitives::AssetId::Asset(asset_id),
		polkadex_primitives::AssetId::Polkadex
	));
	assert_ok!(AssetConversion::add_liquidity(
		RuntimeOrigin::signed(admin),
		polkadex_primitives::AssetId::Asset(asset_id),
		polkadex_primitives::AssetId::Polkadex,
		1_000_000_000_000_000_000_000u128,
		10_000_000_000_000_000u128,
		1u128,
		1u128,
		admin
	));
}
