// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! Tests for pallet-ocex.

use crate::*;
use frame_support::{
	assert_noop, assert_ok, bounded_vec,
	traits::OnInitialize,
};
use polkadex_primitives::{
	assets::AssetId, ingress::IngressMessages, withdrawal::Withdrawal,
	SnapshotAccLimit,
};
use rust_decimal::prelude::FromPrimitive;
use sp_application_crypto::sp_core::H256;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::mock::*;
use codec::Encode;
use frame_system::EventRecord;
use polkadex_primitives::{
	snapshot::{EnclaveSnapshot, Fees},
	AccountId, AssetsLimit, WithdrawalLimit,
};
use rust_decimal::Decimal;
use sp_application_crypto::RuntimePublic;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{
	AccountId32, BoundedBTreeMap, BoundedVec,
	DispatchError::BadOrigin,
	TokenError,
};
use std::sync::Arc;

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

#[test]
fn test_register_main_account() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), false);
		assert_ok!(OCEX::register_main_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		let account_info = Accounts::<Test>::get(account_id.clone()).unwrap();
		assert_eq!(account_info.proxies.len(), 1);
		assert_eq!(account_info.proxies[0], account_id.clone());
		assert_last_event::<Test>(
			crate::Event::MainAccountRegistered {
				main: account_id.clone(),
				proxy: account_id.clone(),
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::RegisterUser(account_id.clone(), account_id.clone());
		assert_eq!(OCEX::ingress_messages()[0], event);
	});
}

#[test]
fn test_register_main_account_main_account_already_exists() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		assert_noop!(
			OCEX::register_main_account(
				Origin::signed(account_id.clone().into()),
				account_id.clone().into()
			),
			Error::<Test>::MainAccountAlreadyRegistered
		);
	});
}

#[test]
fn test_register_main_account_bad_origin() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_main_account(Origin::root(), account_id.clone().into()),
			BadOrigin
		);
		assert_noop!(
			OCEX::register_main_account(Origin::none(), account_id.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_add_proxy_account_main_account_not_found() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(Origin::signed(account_id.clone().into()), account_id.into()),
			Error::<Test>::MainAccountNotFound
		);
	});
}

#[test]
fn test_add_proxy_account_proxy_limit_exceeded() {
	let account_id = create_account_id();
	let proxy_account = create_proxy_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_noop!(
			OCEX::add_proxy_account(
				Origin::signed(account_id.clone().into()),
				proxy_account.clone().into()
			),
			Error::<Test>::ProxyLimitExceeded
		);
	})
}

#[test]
fn test_add_proxy_account_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::add_proxy_account(Origin::root(), account_id.clone().into()), BadOrigin);

		assert_noop!(OCEX::add_proxy_account(Origin::none(), account_id.clone().into()), BadOrigin);
	});
}

#[test]
fn test_add_proxy_account() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			Origin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_last_event::<Test>(
			crate::Event::MainAccountRegistered {
				main: account_id.clone(),
				proxy: account_id.clone(),
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::AddProxy(account_id.clone(), account_id.clone());
		assert_eq!(OCEX::ingress_messages()[1], event);
	});
}

#[test]
fn test_register_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_trading_pair(
				Origin::root(),
				AssetId::polkadex,
				AssetId::polkadex,
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::BothAssetsCannotBeSame
		);
	});
}

#[test]
fn test_register_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_trading_pair(
				Origin::none(),
				AssetId::polkadex,
				AssetId::polkadex,
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			BadOrigin
		);

		assert_noop!(
			OCEX::register_trading_pair(
				Origin::signed(account_id.into()),
				AssetId::polkadex,
				AssetId::polkadex,
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			BadOrigin
		);
	});
}

#[test]
fn test_register_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_trading_pair(
			Origin::root(),
			AssetId::asset(10),
			AssetId::asset(20),
			1_u128.into(),
			100_u128.into(),
			1_u128.into(),
			100_u128.into(),
			100_u128.into(),
			10_u128.into()
		));

		assert_eq!(
			TradingPairs::<Test>::contains_key(AssetId::asset(10), AssetId::asset(20)),
			true
		);
		assert_eq!(TradingPairsStatus::<Test>::get(AssetId::asset(10), AssetId::asset(20)), true);
		assert_last_event::<Test>(
			crate::Event::TradingPairRegistered {
				base: AssetId::asset(10),
				quote: AssetId::asset(20),
			}
			.into(),
		);
		let trading_pair =
			TradingPairs::<Test>::get(AssetId::asset(10), AssetId::asset(20)).unwrap();
		let event: IngressMessages<AccountId32> = IngressMessages::OpenTradingPair(trading_pair);
		assert_eq!(OCEX::ingress_messages()[0], event);
	});
}

#[test]
fn test_register_trading_pair_trading_pair_already_registered() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_trading_pair(
			Origin::root(),
			AssetId::asset(10),
			AssetId::asset(20),
			1_u128.into(),
			100_u128.into(),
			1_u128.into(),
			100_u128.into(),
			100_u128.into(),
			10_u128.into()
		));

		assert_noop!(
			OCEX::register_trading_pair(
				Origin::root(),
				AssetId::asset(10),
				AssetId::asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::TradingPairAlreadyRegistered
		);

		assert_noop!(
			OCEX::register_trading_pair(
				Origin::root(),
				AssetId::asset(20),
				AssetId::asset(10),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::TradingPairAlreadyRegistered
		);
	});
}

#[test]
fn test_deposit_unknown_asset() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::deposit(
				Origin::signed(account_id.clone().into()),
				AssetId::asset(10),
				100_u128.into()
			),
			TokenError::UnknownAsset
		);
	});
}

#[test]
fn test_deposit_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::deposit(Origin::root(), AssetId::asset(10), 100_u128.into()), BadOrigin);

		assert_noop!(OCEX::deposit(Origin::none(), AssetId::asset(10), 100_u128.into()), BadOrigin);
	});
}

#[test]
fn test_deposit() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_custodian_account();
	new_test_ext().execute_with(|| {
		mint_into_account(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000000000000000000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		assert_ok!(OCEX::deposit(
			Origin::signed(account_id.clone().into()),
			AssetId::polkadex,
			100_u128.into()
		));
		// Balances after deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			9999999999999999999900
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 100);
		assert_last_event::<Test>(
			crate::Event::DepositSuccessful {
				user: account_id.clone(),
				asset: AssetId::polkadex,
				amount: 100_u128,
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::Deposit(account_id, AssetId::polkadex, Decimal::new(10, 11));
		assert_eq!(OCEX::ingress_messages()[0], event);
	});
}

#[test]
fn test_deposit_large_value() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_custodian_account();
	new_test_ext().execute_with(|| {
		mint_into_account_large(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			1_000_000_000_000_000_000_000_000_000_000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		assert_noop!(
			OCEX::deposit(
				Origin::signed(account_id.clone().into()),
				AssetId::polkadex,
				1_000_000_000_000_000_000_000_000_0000
			),
			Error::<Test>::DepositOverflow
		);
	});
}

#[test]
fn test_deposit_assets_overflow() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_custodian_account();
	new_test_ext().execute_with(|| {
		mint_into_account_large(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			1_000_000_000_000_000_000_000_000_000_000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		assert_ok!(OCEX::deposit(
			Origin::signed(account_id.clone().into()),
			AssetId::polkadex,
			1_000_000_000_000_000_000_000_000_000
		));
		let large_value: Decimal = Decimal::MAX;
		mint_into_account_large(account_id.clone());
		// Directly setting the storage value, found it very difficult to manually fill it up
		TotalAssets::<Test>::insert(
			AssetId::polkadex,
			large_value.saturating_sub(Decimal::from_u128(1).unwrap()),
		);

		assert_noop!(
			OCEX::deposit(
				Origin::signed(account_id.clone().into()),
				AssetId::polkadex,
				10_u128.pow(20)
			),
			Error::<Test>::DepositOverflow
		);
	});
}

#[test]
fn test_open_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(Origin::root(), AssetId::asset(10), AssetId::asset(10)),
			Error::<Test>::BothAssetsCannotBeSame
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_open_trading_pair_trading_pair_not_found() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(Origin::root(), AssetId::asset(10), AssetId::asset(20)),
			Error::<Test>::TradingPairNotFound
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_open_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(Origin::none(), AssetId::asset(10), AssetId::asset(20)),
			BadOrigin
		);

		assert_noop!(
			OCEX::open_trading_pair(
				Origin::signed(account_id.into()),
				AssetId::asset(10),
				AssetId::asset(20)
			),
			BadOrigin
		);
	});
}

#[test]
fn test_open_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_trading_pair(
			Origin::root(),
			AssetId::asset(10),
			AssetId::asset(20),
			1_u128.into(),
			100_u128.into(),
			1_u128.into(),
			100_u128.into(),
			100_u128.into(),
			10_u128.into()
		));
		assert_ok!(OCEX::open_trading_pair(Origin::root(), AssetId::asset(10), AssetId::asset(20)));
		assert_eq!(TradingPairsStatus::<Test>::get(AssetId::asset(10), AssetId::asset(20)), true);
		let trading_pair = OCEX::trading_pairs(AssetId::asset(10), AssetId::asset(20)).unwrap();
		assert_last_event::<Test>(
			crate::Event::OpenTradingPair { pair: trading_pair.clone() }.into(),
		);
		let event: IngressMessages<AccountId32> = IngressMessages::OpenTradingPair(trading_pair);
		assert_eq!(OCEX::ingress_messages()[0], event);
	})
}

#[test]
fn test_close_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(Origin::root(), AssetId::asset(10), AssetId::asset(10)),
			Error::<Test>::BothAssetsCannotBeSame
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_close_trading_trading_pair_not_found() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(Origin::root(), AssetId::asset(10), AssetId::asset(20)),
			Error::<Test>::TradingPairNotFound
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_close_trading_trading_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(Origin::none(), AssetId::asset(10), AssetId::asset(20)),
			BadOrigin
		);

		assert_noop!(
			OCEX::close_trading_pair(
				Origin::signed(account_id.into()),
				AssetId::asset(10),
				AssetId::asset(20)
			),
			BadOrigin
		);
	});
}

#[test]
fn test_close_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_trading_pair(
			Origin::root(),
			AssetId::asset(10),
			AssetId::asset(20),
			1_u128.into(),
			100_u128.into(),
			1_u128.into(),
			100_u128.into(),
			100_u128.into(),
			10_u128.into()
		));
		assert_ok!(OCEX::close_trading_pair(
			Origin::root(),
			AssetId::asset(10),
			AssetId::asset(20)
		));
		assert_eq!(TradingPairsStatus::<Test>::get(AssetId::asset(10), AssetId::asset(20)), false);
		let trading_pair = OCEX::trading_pairs(AssetId::asset(10), AssetId::asset(20)).unwrap();
		assert_last_event::<Test>(
			crate::Event::ShutdownTradingPair { pair: trading_pair.clone() }.into(),
		);
		let event: IngressMessages<AccountId32> = IngressMessages::CloseTradingPair(trading_pair);
		assert_eq!(OCEX::ingress_messages()[1], event);
	})
}

#[test]
fn collect_fees_unexpected_behaviour() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		assert_ok!(OCEX::collect_fees(
			Origin::signed(account_id.clone().into()),
			100,
			account_id.clone().into()
		));

		assert_last_event::<Test>(
			crate::Event::FeesClaims { beneficiary: account_id, snapshot_id: 100 }.into(),
		);
	});
}

#[test]
fn collect_fees() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_custodian_account();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let public_key_store = KeyStore::new();
	let public_key = SyncCryptoStore::sr25519_generate_new(
		&public_key_store,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(public_key_store)));
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());
		// Initial Balances
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000000000000000000
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			10000000000000000000000
		);
		let fees = create_fees::<Test>();

		let mmr_root: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 1,
				merkle_root: mmr_root,
				withdrawals: Default::default(),
				fees: bounded_vec![fees],
			};
		assert_ok!(OCEX::insert_enclave(Origin::root(), account_id.clone().into()));
		let bytes = snapshot.encode();
		let signature = public_key.sign(KEY_TYPE, &bytes).unwrap();

		assert_ok!(OCEX::submit_snapshot(
			Origin::signed(account_id.clone().into()),
			snapshot,
			signature.clone().into()
		),);

		assert_ok!(OCEX::collect_fees(
			Origin::signed(account_id.clone().into()),
			1,
			account_id.clone().into()
		));
		// Balances after collect fees
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000010000000000000
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			9999999990000000000000
		);
	});
}

#[test]
fn test_collect_fees_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::collect_fees(Origin::root(), 100, account_id.clone().into()), BadOrigin);

		assert_noop!(OCEX::collect_fees(Origin::none(), 100, account_id.into()), BadOrigin);
	});
}

// P.S. This was to apply a DDOS attack and see the response in the mock environment
/* #[test]
fn collect_fees_ddos(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		for x in 0..10000000 {
			assert_ok!(
				OCEX::collect_fees(
					Origin::signed(account_id.clone().into()),
					x,
					account_id.clone().into()
				)
			);
		}
	});
} */

#[test]
fn test_submit_snapshot_sender_is_not_attested_enclave() {
	let account_id = create_account_id();
	let payl: [u8; 64] = [0; 64];
	let sig = sp_application_crypto::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(|| {
		let mmr_root: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 1,
				merkle_root: mmr_root,
				withdrawals: Default::default(),
				fees: bounded_vec![],
			};
		assert_noop!(
			OCEX::submit_snapshot(Origin::signed(account_id.into()), snapshot, sig.clone().into()),
			Error::<Test>::SenderIsNotAttestedEnclave
		);
		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_submit_snapshot_snapshot_nonce_error() {
	let account_id = create_account_id();
	let payl: [u8; 64] = [0; 64];
	let sig = sp_application_crypto::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(|| {
		let mmr_root: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 2,
				merkle_root: mmr_root,
				withdrawals: Default::default(),
				fees: bounded_vec![],
			};
		assert_ok!(OCEX::insert_enclave(Origin::root(), account_id.clone().into()));
		assert_noop!(
			OCEX::submit_snapshot(Origin::signed(account_id.into()), snapshot, sig.clone().into()),
			Error::<Test>::SnapshotNonceError
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_submit_snapshot_enclave_signature_verification_failed() {
	let account_id = create_account_id();
	let payl: [u8; 64] = [0; 64];
	let sig = sp_application_crypto::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(|| {
		let mmr_root: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 1,
				merkle_root: mmr_root,
				withdrawals: Default::default(),
				fees: bounded_vec![],
			};
		assert_ok!(OCEX::insert_enclave(Origin::root(), account_id.clone().into()));
		assert_noop!(
			OCEX::submit_snapshot(Origin::signed(account_id.into()), snapshot, sig.clone().into()),
			Error::<Test>::EnclaveSignatureVerificationFailed
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_submit_snapshot_bad_origin() {
	let payl: [u8; 64] = [0; 64];
	let sig = sp_application_crypto::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(|| {
		let mmr_root: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 0,
				merkle_root: mmr_root,
				withdrawals: Default::default(),
				fees: bounded_vec![],
			};
		assert_noop!(
			OCEX::submit_snapshot(Origin::root(), snapshot.clone(), sig.clone().into()),
			BadOrigin
		);

		assert_noop!(
			OCEX::submit_snapshot(Origin::root(), snapshot, sig.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_submit_snapshot() {
	let account_id = create_account_id();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let public_key_store = KeyStore::new();
	let public_key = SyncCryptoStore::sr25519_generate_new(
		&public_key_store,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(public_key_store)));
	t.execute_with(|| {
		let withdrawal = create_withdrawal::<Test>();
		let mmr_root: H256 = H256::random();
		let mut withdrawal_map: BoundedBTreeMap<
			AccountId,
			BoundedVec<Withdrawal<AccountId>, WithdrawalLimit>,
			SnapshotAccLimit,
		> = BoundedBTreeMap::new();
		withdrawal_map.try_insert(account_id.clone(), bounded_vec![withdrawal]).unwrap();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 1,
				merkle_root: mmr_root,
				withdrawals: withdrawal_map.clone(),
				fees: bounded_vec![],
			};
		assert_ok!(OCEX::insert_enclave(Origin::root(), account_id.clone().into()));
		let bytes = snapshot.encode();
		let signature = public_key.sign(KEY_TYPE, &bytes).unwrap();

		assert_ok!(OCEX::submit_snapshot(
			Origin::signed(account_id.into()),
			snapshot.clone(),
			signature.clone().into()
		),);
		assert_eq!(Withdrawals::<Test>::contains_key(1), true);
		assert_eq!(Withdrawals::<Test>::get(1), withdrawal_map.clone());
		assert_eq!(FeesCollected::<Test>::contains_key(1), true);
		assert_eq!(Snapshots::<Test>::contains_key(1), true);
		assert_eq!(Snapshots::<Test>::get(1).unwrap(), snapshot.clone());
		assert_eq!(SnapshotNonce::<Test>::get().unwrap(), 1);
		let onchain_events: BoundedVec<
			polkadex_primitives::ocex::OnChainEvents<AccountId>,
			polkadex_primitives::OnChainEventsLimit,
		> = bounded_vec![polkadex_primitives::ocex::OnChainEvents::GetStorage(
			polkadex_primitives::ocex::Pallet::OCEX,
			polkadex_primitives::ocex::StorageItem::Withdrawal,
			1
		)];
		assert_eq!(OnChainEvents::<Test>::get(), onchain_events);
	})
}

#[test]
fn test_register_enclave() {
	let account_id = create_account_id();
	let ias_report = vec![
		19, 19, 2, 7, 255, 128, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 7, 0,
		0, 0, 0, 0, 0, 0, 157, 113, 31, 38, 134, 1, 92, 170, 202, 207, 84, 214, 193, 115, 135, 89,
		228, 23, 80, 184, 116, 61, 170, 171, 159, 47, 5, 32, 99, 126, 11, 13, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 183, 141,
		57, 75, 101, 149, 246, 85, 227, 219, 71, 14, 143, 143, 79, 2, 209, 127, 165, 117, 206, 185,
		73, 81, 228, 1, 225, 150, 116, 242, 38, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 95,
		159, 233, 74, 113, 162, 222, 24, 218, 134, 159, 15, 74, 157, 188, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 69, 66, 236, 163, 63, 254, 74, 251, 172, 254, 123, 233, 19, 175,
		193, 204,
	];
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_enclave(Origin::signed(account_id.clone()), ias_report),
			Error::<Test>::RemoteAttestationVerificationFailed
		);
	});
}

#[test]
fn test_register_enclave_empty_report() {
	let account_id = create_account_id();
	let ias_report = vec![];
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_enclave(Origin::signed(account_id), ias_report),
			Error::<Test>::RemoteAttestationVerificationFailed
		);
	});
}

#[test]
fn test_reigster_enclave_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::register_enclave(Origin::root(), vec![]), BadOrigin);

		assert_noop!(OCEX::register_enclave(Origin::none(), vec![]), BadOrigin);
	});
}

#[test]
fn test_withdrawal_invalid_withdrawal_index() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::withdraw(Origin::signed(account_id.clone().into()), 1,),
			Error::<Test>::InvalidWithdrawalIndex
		);
	});
}

#[test]
fn test_withdrawal() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_custodian_account();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let public_key_store = KeyStore::new();
	let public_key = SyncCryptoStore::sr25519_generate_new(
		&public_key_store,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(public_key_store)));
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());
		// Initial Balances
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000000000000000000
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			10000000000000000000000
		);
		let withdrawal = create_withdrawal::<Test>();
		let mut withdrawal_map: BoundedBTreeMap<
			AccountId,
			BoundedVec<Withdrawal<AccountId>, WithdrawalLimit>,
			SnapshotAccLimit,
		> = BoundedBTreeMap::new();
		withdrawal_map.try_insert(account_id.clone(), bounded_vec![withdrawal.clone()]).unwrap();

		let mmr_root: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 1,
				merkle_root: mmr_root,
				withdrawals: withdrawal_map,
				fees: bounded_vec![],
			};
		assert_ok!(OCEX::insert_enclave(Origin::root(), account_id.clone().into()));
		let bytes = snapshot.encode();
		let signature = public_key.sign(KEY_TYPE, &bytes).unwrap();

		assert_ok!(OCEX::submit_snapshot(
			Origin::signed(account_id.clone().into()),
			snapshot,
			signature.clone().into()
		),);

		assert_ok!(OCEX::withdraw(Origin::signed(account_id.clone().into()), 1,));
		// Balances after withdrawal
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000100000000000000
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			9999999900000000000000,
		);
		let withdrawal_claimed: polkadex_primitives::ocex::OnChainEvents<AccountId> =
			polkadex_primitives::ocex::OnChainEvents::OrderBookWithdrawalClaimed(
				1,
				account_id.clone().into(),
				bounded_vec![withdrawal],
			);
		assert_eq!(OnChainEvents::<Test>::get()[1], withdrawal_claimed);
	});
}
#[test]
fn test_onchain_events_overflow() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_custodian_account();
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let public_key_store = KeyStore::new();
	let public_key = SyncCryptoStore::sr25519_generate_new(
		&public_key_store,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	// create 500 accounts
	let mut account_id_vector: Vec<AccountId> = vec![];
	for x in 0..500 {
		let account_id_500 = create_account_id_500(x as u32);
		account_id_vector.push(account_id_500);
	}
	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(public_key_store)));
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());
		let withdrawal = create_withdrawal::<Test>();
		let mut withdrawal_map: BoundedBTreeMap<
			AccountId,
			BoundedVec<Withdrawal<AccountId>, WithdrawalLimit>,
			SnapshotAccLimit,
		> = BoundedBTreeMap::new();
		withdrawal_map.try_insert(account_id.clone(), bounded_vec![withdrawal.clone()]).unwrap();
		for x in account_id_vector.clone() {
			withdrawal_map.try_insert(x, bounded_vec![withdrawal.clone()]).unwrap();
		}

		let hash: H256 = H256::random();
		let snapshot =
			EnclaveSnapshot::<AccountId32, WithdrawalLimit, AssetsLimit, SnapshotAccLimit> {
				snapshot_number: 1,
				merkle_root: hash,
				withdrawals: withdrawal_map,
				fees: bounded_vec![],
			};
		assert_ok!(OCEX::insert_enclave(Origin::root(), account_id.clone().into()));
		let bytes = snapshot.encode();
		let signature = public_key.sign(KEY_TYPE, &bytes).unwrap();

		assert_ok!(OCEX::submit_snapshot(
			Origin::signed(account_id.clone().into()),
			snapshot,
			signature.clone().into()
		),);

		// Perform withdraw for 500 accounts
		for x in 0..account_id_vector.len() - 1 {
			assert_ok!(OCEX::withdraw(Origin::signed(account_id_vector[x].clone().into()), 1));
		}
		let last_account = account_id_vector.len() - 1;
		assert_noop!(
			OCEX::withdraw(Origin::signed(account_id_vector[last_account].clone().into()), 1),
			Error::<Test>::OnchainEventsBoundedVecOverflow
		);

		// Cleanup Onchain events
		<OCEX as OnInitialize<u64>>::on_initialize(0);
		assert_eq!(OnChainEvents::<Test>::get().len(), 0);

		// Perform withdraw now
		assert_ok!(OCEX::withdraw(
			Origin::signed(account_id_vector[last_account].clone().into()),
			1
		));
	});
}

#[test]
fn test_withdrawal_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::withdraw(Origin::root(), 1,), BadOrigin);

		assert_noop!(OCEX::withdraw(Origin::none(), 1,), BadOrigin);
	});
}

#[test]
fn test_shutdown() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::shutdown(Origin::root()));

		let ingress_message: IngressMessages<AccountId32> = IngressMessages::Shutdown;
		assert_eq!(OCEX::ingress_messages()[0], ingress_message);
		assert_eq!(ExchangeState::<Test>::get(), false);
	});
}

#[test]
fn test_shutdown_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::shutdown(Origin::signed(account_id.into())), BadOrigin);

		assert_noop!(OCEX::shutdown(Origin::none()), BadOrigin);
	});
}

fn mint_into_account(account_id: AccountId32) {
	let _result = Balances::deposit_creating(&account_id, 10000000000000000000000);
}

fn mint_into_account_large(account_id: AccountId32) {
	let _result = Balances::deposit_creating(&account_id, 1_000_000_000_000_000_000_000_000_000_000);
}

#[allow(dead_code)]
fn create_asset_and_credit(asset_id: u128, account_id: AccountId32) {
	assert_ok!(Assets::create(
		Origin::signed(account_id.clone().into()),
		asset_id.into(),
		account_id.clone().into(),
		100_u128
	));
}

fn create_account_id() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
}
fn create_account_id_500(uid: u32) -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter{}", PHRASE, uid)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
}

fn create_proxy_account() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter2", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id
}

#[allow(dead_code)]
fn create_public_key() -> sp_application_crypto::sr25519::Public {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");

	return account_id
}

pub fn create_withdrawal<T: Config>() -> Withdrawal<AccountId32> {
	let account_id = create_account_id();
	let withdrawal: Withdrawal<AccountId32> = Withdrawal {
		main_account: account_id,
		asset: AssetId::polkadex,
		amount: 100_u32.into(),
		event_id: 0,
		fees: 1_u32.into(),
	};
	return withdrawal
}

pub fn create_fees<T: Config>() -> Fees {
	let fees: Fees = Fees { asset: AssetId::polkadex, amount: Decimal::new(100, 1) };
	return fees
}
