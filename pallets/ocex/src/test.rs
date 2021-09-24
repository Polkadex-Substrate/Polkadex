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
use polkadex_primitives::AccountId;

const GEN_ACCOUNT: AccountId = AccountId::new(*b"01234567890123456789012345678901");
const NEW_ACCOUNT: AccountId = AccountId::new(*b"12345678901234567890123456789012");
const NEW_ACCOUNT_TWO: AccountId = AccountId::new(*b"23456789012345678901234567890123");
const PROXY_ACCOUNT_ONE: AccountId = AccountId::new(*b"34567890123456789012345678901234");
const PROXY_ACCOUNT_TWO: AccountId = AccountId::new(*b"45678901234567890123456789012345");
const PROXY_ACCOUNT_THREE: AccountId = AccountId::new(*b"56789012345678901234567890123456");
const NOT_REGISTERED_ACCOUNT: AccountId = AccountId::new(*b"67890123456789012345678901234567");
const OCEX_ACCOUNT_ID: AccountId = AccountId::new(*b"67890123456789012345678901234590");

#[test]
fn test_register_account() {
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        // Register new account
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone()
        ));
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT_TWO.clone()),
            NEW_ACCOUNT_TWO.clone()
        ));

        // Verify LastAccount Storage
        assert_eq!(<LastAccount<Test>>::get(), NEW_ACCOUNT_TWO);
        // Verify Main Account Storage
        let latest_linked_account: LinkedAccount = LinkedAccount {
            prev: NEW_ACCOUNT.clone(),
            current: NEW_ACCOUNT_TWO.clone(),
            next: None,
            proxies: vec![],
        };
        let linked_account: LinkedAccount = LinkedAccount {
            prev: GEN_ACCOUNT.clone(),
            current: NEW_ACCOUNT.clone(),
            next: Some(NEW_ACCOUNT_TWO.clone()),
            proxies: vec![],
        };
        let expected_linked_account_gen: LinkedAccount = LinkedAccount {
            prev: GEN_ACCOUNT.clone(),
            current: GEN_ACCOUNT.clone(),
            next: Some(NEW_ACCOUNT.clone()),
            proxies: vec![],
        };
        assert_eq!(
            <MainAccounts<Test>>::get(NEW_ACCOUNT_TWO),
            latest_linked_account
        );
        assert_eq!(<MainAccounts<Test>>::get(NEW_ACCOUNT), linked_account);
        assert_eq!(
            <MainAccounts<Test>>::get(GEN_ACCOUNT),
            expected_linked_account_gen
        );
    });

    // Test Errors
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone()
        ));
        assert_noop!(
            PolkadexOcexPallet::register(Origin::signed(NEW_ACCOUNT.clone()), NEW_ACCOUNT),
            Error::<Test>::AlreadyRegistered
        );
    });
}

#[test]
fn test_add_proxy() {
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone()
        ));
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone(),
            PROXY_ACCOUNT_ONE.clone()
        ));
        // TODO: Already registered Proxies can be registered multiple times
        //assert_ok!(PolkadexOcexPallet::add_proxy(Origin::signed(NEW_ACCOUNT), PROXY_ACCOUNT_ONE));
        let expected_linked_account: LinkedAccount = LinkedAccount {
            prev: GEN_ACCOUNT,
            current: NEW_ACCOUNT.clone(),
            next: None,
            proxies: vec![PROXY_ACCOUNT_ONE],
        };
        assert_eq!(
            <MainAccounts<Test>>::get(NEW_ACCOUNT),
            expected_linked_account
        );
    });

    // Test Errors
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone()
        ));
        assert_noop!(
            PolkadexOcexPallet::add_proxy(
                Origin::signed(NOT_REGISTERED_ACCOUNT.clone()),
                NOT_REGISTERED_ACCOUNT,
                PROXY_ACCOUNT_ONE.clone()
            ),
            Error::<Test>::NotARegisteredMainAccount
        );
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone(),
            PROXY_ACCOUNT_ONE
        ));
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone(),
            PROXY_ACCOUNT_TWO
        ));

        // Check proxy Limit
        assert_noop!(
            PolkadexOcexPallet::add_proxy(
                Origin::signed(NEW_ACCOUNT.clone()),
                NEW_ACCOUNT,
                PROXY_ACCOUNT_THREE
            ),
            Error::<Test>::ProxyLimitReached
        );
    });
}

#[test]
fn test_remove_proxy() {
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone()
        ));
        assert_ok!(PolkadexOcexPallet::add_proxy(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone(),
            PROXY_ACCOUNT_ONE.clone()
        ));
        assert_ok!(PolkadexOcexPallet::remove_proxy(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT.clone(),
            PROXY_ACCOUNT_ONE
        ));

        let expected_linked_account: LinkedAccount = LinkedAccount {
            prev: GEN_ACCOUNT,
            current: NEW_ACCOUNT.clone(),
            next: None,
            proxies: vec![],
        };
        assert_eq!(
            <MainAccounts<Test>>::get(NEW_ACCOUNT),
            expected_linked_account
        );
    });

    // Verify Errors
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        assert_ok!(PolkadexOcexPallet::register(
            Origin::signed(NEW_ACCOUNT.clone()),
            NEW_ACCOUNT
        ));
        assert_noop!(
            PolkadexOcexPallet::remove_proxy(
                Origin::signed(NOT_REGISTERED_ACCOUNT.clone()),
                NOT_REGISTERED_ACCOUNT,
                PROXY_ACCOUNT_ONE
            ),
            Error::<Test>::NotARegisteredMainAccount
        );
    });
}

#[test]
fn test_upload_cid() {
    // Happy Path
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        pallet_substratee_registry::EnclaveIndex::<Test>::insert(OCEX_ACCOUNT_ID, 0u64);
        let cid: Vec<u8> = vec![0];
        assert_ok!(PolkadexOcexPallet::upload_cid(
            Origin::signed(OCEX_ACCOUNT_ID.clone()),
            cid.clone()
        ));
        assert_eq!(<Snapshot<Test>>::get(OCEX_ACCOUNT_ID), cid);

        // Modify Data
        let new_cid: Vec<u8> = vec![1];
        assert_ok!(PolkadexOcexPallet::upload_cid(
            Origin::signed(OCEX_ACCOUNT_ID.clone()),
            new_cid.clone()
        ));
        assert_eq!(<Snapshot<Test>>::get(OCEX_ACCOUNT_ID), new_cid);
    });

    //Test Error
    new_test_ext(GEN_ACCOUNT).execute_with(|| {
        // NotARegisteredEnclave
        let cid: Vec<u8> = vec![1];
        assert_noop!(
            PolkadexOcexPallet::upload_cid(Origin::signed(OCEX_ACCOUNT_ID.clone()), cid),
            Error::<Test>::NotARegisteredEnclave
        );
    });
}
