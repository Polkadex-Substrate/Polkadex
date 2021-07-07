// // This file is part of Polkadex.
//
// // Copyright (C) 2020-2021 Polkadex oü.
// // SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// // This program is free software: you can redistribute it and/or modify
// // it under the terms of the GNU General Public License as published by
// // the Free Software Foundation, either version 3 of the License, or
// // (at your option) any later version.
//
// // This program is distributed in the hope that it will be useful,
// // but WITHOUT ANY WARRANTY; without even the implied warranty of
// // MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// // GNU General Public License for more details.
//
// // You should have received a copy of the GNU General Public License
// // along with this program. If not, see <https://www.gnu.org/licenses/>.
//
use crate::mock::*;
use frame_support::{assert_noop, assert_ok};

use super::*;
use orml_traits::MultiCurrency;
use polkadex_primitives::assets::AssetId;
use sp_core::H160;

#[test]
fn test_register_account() {
    new_test_ext(0).execute_with(|| {
        // Register new account
        let new_account: u64 = 2;
        let gen_account: u64 = 0;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account),
            new_account
        ));
        let new_account_two: u64 = 3;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account_two),
            new_account_two
        ));

        // Verify LastAccount Storage
        assert_eq!(<LastAccount<Test>>::get(), 3);
        // Verify Main Account Storage
        let latest_linked_account: LinkedAccount<Test> = LinkedAccount {
            prev: new_account,
            current: new_account_two,
            next: None,
            proxies: vec![],
        };
        let linked_account: LinkedAccount<Test> = LinkedAccount {
            prev: gen_account,
            current: new_account,
            next: Some(new_account_two),
            proxies: vec![],
        };
        let expected_linked_account_gen: LinkedAccount<Test> = LinkedAccount {
            prev: gen_account,
            current: gen_account,
            next: Some(new_account),
            proxies: vec![],
        };
        assert_eq!(
            <MainAccounts<Test>>::get(new_account_two),
            latest_linked_account
        );
        assert_eq!(<MainAccounts<Test>>::get(new_account), linked_account);
        assert_eq!(
            <MainAccounts<Test>>::get(gen_account),
            expected_linked_account_gen
        );
    });

    // Test Errors
    new_test_ext(0).execute_with(|| {
        let new_account: u64 = 2;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account),
            new_account
        ));
        assert_noop!(
            PolkadexOcexPallet::register(Origin::signed(2), 2u64),
            Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn test_add_proxy() {
    new_test_ext(0).execute_with(|| {
        let new_account: u64 = 2;
        let gen_account: u64 = 0;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account),
            new_account
        ));
        let proxy_account_one = 3;
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(new_account),
            new_account,
            proxy_account_one
        ));
        // TODO: Already registered Proxies can be registered multiple times
        //assert_ok!(PolkadexOcexPallet::add_proxy(Origin::signed(new_account), proxy_account_one));
        let expected_linked_account: LinkedAccount<Test> = LinkedAccount {
            prev: gen_account,
            current: new_account,
            next: None,
            proxies: vec![3],
        };
        assert_eq!(
            <MainAccounts<Test>>::get(new_account),
            expected_linked_account
        );
    });

    // Test Errors
    new_test_ext(0).execute_with(|| {
        let new_account: u64 = 2;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account),
            new_account
        ));
        let proxy_account_one = 3;
        let proxy_account_two = 4;
        let not_registered_account: u64 = 4;
        assert_noop!(
            PolkadexOcexPallet::add_proxy(
                Origin::signed(not_registered_account),
                not_registered_account,
                proxy_account_one
            ),
            Error::<Test>::NotARegisteredMainAccount
        );
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(new_account),
            new_account,
            proxy_account_one
        ));
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(new_account),
            new_account,
            proxy_account_two
        ));

        // Check proxy Limit
        let proxy_account_three = 5;
        assert_noop!(
            PolkadexOcexPallet::add_proxy(
                Origin::signed(new_account),
                new_account,
                proxy_account_three
            ),
            Error::<Test>::ProxyLimitReached
        );
    });
}

#[test]
fn test_remove_proxy() {
    new_test_ext(0).execute_with(|| {
        let new_account: u64 = 2;
        let gen_account: u64 = 0;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account),
            new_account
        ));
        let proxy_account_one = 3;
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(new_account),
            new_account,
            proxy_account_one
        ));
        assert_ok!(PolkadexOcexPallet::remove_proxy(
            Origin::signed(new_account),
            new_account,
            proxy_account_one
        ));
        let expected_linked_account: LinkedAccount<Test> = LinkedAccount {
            prev: gen_account,
            current: new_account,
            next: None,
            proxies: vec![],
        };
        assert_eq!(
            <MainAccounts<Test>>::get(new_account),
            expected_linked_account
        );
    });

    // Verify Errors
    new_test_ext(0).execute_with(|| {
        let new_account: u64 = 2;
        let proxy_account_one = 3;
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(new_account),
            new_account
        ));
        let not_registered_account: u64 = 4;
        assert_noop!(
            PolkadexOcexPallet::remove_proxy(
                Origin::signed(not_registered_account),
                not_registered_account,
                proxy_account_one
            ),
            Error::<Test>::NotARegisteredMainAccount
        );
    });
}
