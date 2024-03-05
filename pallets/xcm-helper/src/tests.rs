// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

use crate::{mock::*, Error, PendingWithdrawals};
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::{traits::AccountIdConversion, DispatchError, SaturatedConversion};
use thea_primitives::types::Withdraw;
use xcm::latest::{AssetId, MultiLocation};

#[test]
fn test_whitelist_token_returns_ok() {
    new_test_ext().execute_with(|| {
        let asset_location = MultiLocation::parent();
        let token: AssetId = AssetId::Concrete(asset_location);
        assert_ok!(XcmHelper::whitelist_token(RuntimeOrigin::root(), token));
    });
}

#[test]
fn test_whitelist_token_with_bad_origin_will_return_bad_origin_error() {
    new_test_ext().execute_with(|| {
        let asset_location = MultiLocation::parent();
        let token: AssetId = AssetId::Concrete(asset_location);

        assert_noop!(
            XcmHelper::whitelist_token(RuntimeOrigin::none(), token),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn test_remove_whitelisted_token_returns_ok() {
    new_test_ext().execute_with(|| {
        let asset_location = MultiLocation::parent();
        let token: AssetId = AssetId::Concrete(asset_location);
        assert_ok!(XcmHelper::whitelist_token(RuntimeOrigin::root(), token));
        assert_ok!(XcmHelper::remove_whitelisted_token(
            RuntimeOrigin::root(),
            token
        ));
    });
}

#[test]
fn test_remove_whitelisted_token_returns_token_not_found_error() {
    new_test_ext().execute_with(|| {
        let asset_location = MultiLocation::parent();
        let token: AssetId = AssetId::Concrete(asset_location);
        assert_noop!(
            XcmHelper::remove_whitelisted_token(RuntimeOrigin::root(), token),
            Error::<Test>::TokenIsNotWhitelisted
        );
    });
}

#[test]
fn test_remove_whitelisted_token_with_bad_origin_will_return_bad_origin_error() {
    new_test_ext().execute_with(|| {
        let asset_location = MultiLocation::parent();
        let token: AssetId = AssetId::Concrete(asset_location);

        assert_noop!(
            XcmHelper::remove_whitelisted_token(RuntimeOrigin::none(), token),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn test_whitelist_token_returns_token_is_already_whitelisted() {
    new_test_ext().execute_with(|| {
        let asset_location = MultiLocation::parent();
        let token: AssetId = AssetId::Concrete(asset_location);
        assert_ok!(XcmHelper::whitelist_token(RuntimeOrigin::root(), token));
        assert_noop!(
            XcmHelper::whitelist_token(RuntimeOrigin::root(), token),
            Error::<Test>::TokenIsAlreadyWhitelisted
        );
    });
}

#[test]
fn test_transfer_fee_returns_ok() {
    new_test_ext().execute_with(|| {
        let recipient = 1;
        let pallet_account = AssetHandlerPalletId::get().into_account_truncating();
        let _ = Balances::deposit_creating(
            &pallet_account,
            5_000_000_000_000_000_000_000u128.saturated_into(),
        );
        assert_ok!(XcmHelper::transfer_fee(RuntimeOrigin::root(), recipient));
        assert_eq!(
            Balances::free_balance(recipient),
            4999999999000000000000u128.saturated_into()
        );
    });
}

#[test]
fn test_transfer_fee_with_bad_origin_will_return_bad_origin_error() {
    new_test_ext().execute_with(|| {
        let recipient = 1;
        let pallet_account = AssetHandlerPalletId::get().into_account_truncating();
        let _ = Balances::deposit_creating(
            &pallet_account,
            5_000_000_000_000_000_000_000u128.saturated_into(),
        );

        assert_noop!(
            XcmHelper::transfer_fee(RuntimeOrigin::none(), recipient),
            DispatchError::BadOrigin
        );
    });
}

#[test]
fn test_block_by_ele() {
    new_test_ext().execute_with(|| {
        let first_withdrawal = Withdraw {
            id: Vec::new(),
            asset_id: 1,
            amount: 1,
            destination: vec![],
            is_blocked: false,
            extra: vec![],
        };
        let sec_withdrawal = Withdraw {
            id: Vec::new(),
            asset_id: 2,
            amount: 2,
            destination: vec![],
            is_blocked: false,
            extra: vec![],
        };
        <PendingWithdrawals<Test>>::insert(1, vec![first_withdrawal, sec_withdrawal]);
        assert_ok!(XcmHelper::block_by_ele(1, 1));
        let actual_withdrawals = <PendingWithdrawals<Test>>::get(1);
        let expected_withdraw = Withdraw {
            id: Vec::new(),
            asset_id: 2,
            amount: 2,
            destination: vec![],
            is_blocked: true,
            extra: vec![],
        };
        assert_eq!(actual_withdrawals[1], expected_withdraw);
        assert_noop!(XcmHelper::block_by_ele(1, 4), Error::<Test>::IndexNotFound);
    });
}
