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
	parameter_types,
	traits::{ConstU128, ConstU64},
	PalletId,
	assert_noop, assert_ok,
};
use polkadex_primitives::ingress::IngressMessages;
use frame_support::bounded_vec;
use frame_support::traits::OnTimestampSet;
use polkadex_primitives::{Moment, Signature, assets::AssetId};
use sp_std::cell::RefCell;
use frame_system::EnsureRoot;
use sp_core::H256;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32,
	TokenError,
	DispatchError::BadOrigin
};
use crate::mock::*;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use ckb_merkle_mountain_range::util::MemStore;
use ckb_merkle_mountain_range::{Merge, MMR};
use codec::Encode;
use polkadex_primitives::ocex::AccountInfo;
use polkadex_primitives::{AccountId, Balance, ProxyLimit, WithdrawalLimit, AssetsLimit};
use std::collections::btree_map::Values;
use std::collections::BTreeMap;
use polkadex_primitives::snapshot::EnclaveSnapshot;
use sp_application_crypto::RuntimePublic;
use std::sync::Arc;
use sp_runtime::traits::Verify;
use frame_system::EventRecord;

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

#[test]
fn test_register_main_account(){
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), false);
		assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		assert_last_event::<Test>(crate::Event::MainAccountRegistered{main: account_id.clone(), proxy: account_id.clone()}.into());
		let event: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::RegisterUser(account_id.clone(), account_id.clone());
		assert_eq!(OCEX::ingress_messages()[0], event);
		
	});
}

// TODO! Need to cover limit error 
#[test]
fn test_register_main_account_main_account_already_exists(){
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		assert_noop!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()), Error::<Test>::MainAccountAlreadyRegistered);
	});
}

#[test]
fn test_register_main_account_bad_origin(){
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_noop!(OCEX::register_main_account(Origin::root(), account_id.clone().into()), BadOrigin);
		assert_noop!(OCEX::register_main_account(Origin::none(), account_id.clone().into()), BadOrigin);
	});
}

#[test]
fn test_add_proxy_account_main_account_not_found(){
	let account_id = create_account_id(); 

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(Origin::signed(account_id.clone().into()), account_id.into()),
			Error::<Test>::MainAccountNotFound
		);
	});
}

#[test]
fn test_add_proxy_account_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(Origin::root(), account_id.clone().into()),
			BadOrigin
		);

		assert_noop!(
			OCEX::add_proxy_account(Origin::none(), account_id.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_add_proxy_account(){
	let account_id = create_account_id(); 

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_ok!(OCEX::add_proxy_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_last_event::<Test>(crate::Event::MainAccountRegistered{main: account_id.clone(), proxy: account_id.clone()}.into());
		let event: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::AddProxy(account_id.clone(), account_id.clone());
		assert_eq!(OCEX::ingress_messages()[1], event);
	});
}

#[test]
fn test_register_trading_pair_both_assets_cannot_be_same(){
	new_test_ext().execute_with(||{
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
fn test_register_trading_pair_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
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
fn test_register_trading_pair(){
	new_test_ext().execute_with(||{
		assert_ok!(
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
			)
		);

		assert_eq!(TradingPairs::<Test>::contains_key(AssetId::asset(10), AssetId::asset(20)), true);
		assert_eq!(TradingPairsStatus::<Test>::get(AssetId::asset(10), AssetId::asset(20)), true);
		assert_last_event::<Test>(crate::Event::TradingPairRegistered{base: AssetId::asset(10), quote: AssetId::asset(20)}.into());
		let trading_pair = TradingPairs::<Test>::get(AssetId::asset(10), AssetId::asset(20)).unwrap();
		let event: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::OpenTradingPair(trading_pair);
		assert_eq!(OCEX::ingress_messages()[0], event);
		
	});
}

#[test]
fn test_register_trading_pair_trading_pair_already_registered(){
	new_test_ext().execute_with(||{
		assert_ok!(
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
			)
		);

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
fn test_deposit_unknown_asset(){
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
fn test_deposit_bad_origin(){
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::deposit(
				Origin::root(),
				AssetId::asset(10),
				100_u128.into()
			),
			BadOrigin
		);

		assert_noop!(
			OCEX::deposit(
				Origin::none(),
				AssetId::asset(10),
				100_u128.into()
			), 
			BadOrigin
		);
	});
}

#[test]
fn test_deposit(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		mint_into_account(account_id.clone());
		assert_ok!(
			OCEX::deposit(
				Origin::signed(account_id.clone().into()),
				AssetId::polkadex,
				100_u128.into()
			)
		);
		assert_last_event::<Test>(crate::Event::DepositSuccessful{user: account_id.clone(), asset: AssetId::polkadex, amount: 100_u128}.into());
		let event: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::Deposit(account_id, AssetId::polkadex, 100_u128);
		assert_eq!(OCEX::ingress_messages()[0], event);
	});
}

#[test]
fn test_open_trading_pair_both_assets_cannot_be_same(){
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::open_trading_pair(
				Origin::root(),
				AssetId::asset(10),
				AssetId::asset(10)
			),
			Error::<Test>::BothAssetsCannotBeSame
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_open_trading_pair_trading_pair_not_found(){
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(
				Origin::root(), 
				AssetId::asset(10), 
				AssetId::asset(20)
			),
			Error::<Test>::TradingPairNotFound
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_open_trading_pair_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(
				Origin::none(), 
				AssetId::asset(10), 
				AssetId::asset(20)
			),
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
fn test_open_trading_pair(){
	new_test_ext().execute_with(||{
		assert_ok!(
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
			)
		);
		assert_ok!(
			OCEX::open_trading_pair(
				Origin::root(),
				AssetId::asset(10),
				AssetId::asset(20)
			)
		);
		assert_eq!(
			TradingPairsStatus::<Test>::get(AssetId::asset(10), AssetId::asset(20)), 
			true
		);
		let trading_pair = OCEX::trading_pairs(AssetId::asset(10), AssetId::asset(20)).unwrap();
		assert_last_event::<Test>(crate::Event::OpenTradingPair{pair: trading_pair.clone()}.into());
		let event: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::OpenTradingPair(trading_pair);
		assert_eq!(OCEX::ingress_messages()[0], event);
	})
}

#[test]
fn test_close_trading_pair_both_assets_cannot_be_same(){
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::close_trading_pair(
				Origin::root(),
				AssetId::asset(10),
				AssetId::asset(10)
			),
			Error::<Test>::BothAssetsCannotBeSame
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_close_trading_trading_pair_not_found(){
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::close_trading_pair(
				Origin::root(),
				AssetId::asset(10),
				AssetId::asset(20)
			),
			Error::<Test>::TradingPairNotFound
		);

		assert_eq!(OCEX::ingress_messages().len(), 0);
	});
}

#[test]
fn test_close_trading_trading_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::close_trading_pair(
				Origin::none(),
				AssetId::asset(10),
				AssetId::asset(20)
			),
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
fn test_close_trading_pair(){
	new_test_ext().execute_with(||{
		assert_ok!(
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
			)
		);
		assert_ok!(
			OCEX::close_trading_pair(
				Origin::root(),
				AssetId::asset(10),
				AssetId::asset(20)
			)
		);
		assert_eq!(
			TradingPairsStatus::<Test>::get(AssetId::asset(10), AssetId::asset(20)), 
			false
		);
		let trading_pair = OCEX::trading_pairs(AssetId::asset(10), AssetId::asset(20)).unwrap();
		assert_last_event::<Test>(crate::Event::ShutdownTradingPair{pair: trading_pair.clone()}.into());
		let event: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::CloseTradingPair(trading_pair);
		assert_eq!(OCEX::ingress_messages()[1], event);
	})
}

#[test]
fn collect_fees(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		assert_ok!(
			OCEX::collect_fees(
				Origin::signed(account_id.clone().into()),
				100,
				account_id.clone().into()
			)
		);

		assert_last_event::<Test>(crate::Event::FeesClaims{beneficiary: account_id, snapshot_id: 100}.into());
	});	
}

#[test]
fn test_collect_fees_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::collect_fees(
				Origin::root(),
				100,
				account_id.clone().into()
			), 
			BadOrigin
		);

		assert_noop!(
			OCEX::collect_fees(
				Origin::none(), 
				100, 
				account_id.into()
			), 
			BadOrigin
		);
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
fn test_submit_snapshot_sender_is_not_attested_enclave(){
	let account_id = create_account_id();
	let payl: [u8; 64] = [0; 64];
	let sig = sp_core::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(||{
		let mmr_root: H256 = create_mmr_with_one_account();
		let mut snapshot = EnclaveSnapshot::<AccountId32, Balance, WithdrawalLimit, AssetsLimit>{
			snapshot_number: 1,
    		merkle_root: mmr_root,
			withdrawals: bounded_vec![],
    		fees: bounded_vec![],

		};
		assert_noop!(
			OCEX::submit_snapshot(
				Origin::signed(account_id.into()),
				snapshot,
				sig.clone().into()
			), 
			Error::<Test>::SenderIsNotAttestedEnclave
		);
		// There is an existing ingress message which holds RegisterUser
		assert_eq!(OCEX::ingress_messages().len(), 1);
	});
}

#[test]
fn test_submit_snapshot_snapshot_nonce_error(){
	let account_id = create_account_id();
	let payl: [u8; 64] = [0; 64];
	let sig = sp_core::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(||{
		let mmr_root: H256 = create_mmr_with_one_account();
		let mut snapshot = EnclaveSnapshot::<AccountId32, Balance, WithdrawalLimit, AssetsLimit>{
			snapshot_number: 1,
    		merkle_root: mmr_root,
			withdrawals: bounded_vec![],
    		fees: bounded_vec![],

		};
		assert_ok!(
			OCEX::insert_enclave(
				Origin::root(),
				account_id.clone().into()
			)
		);
		assert_noop!(
			OCEX::submit_snapshot(
				Origin::signed(account_id.into()),
				snapshot,
				sig.clone().into()
			), 
			Error::<Test>::SnapshotNonceError
		);

		assert_eq!(OCEX::ingress_messages().len(), 1);
	});
}

#[test]
fn test_submit_snapshot_enclave_signature_verification_failed(){
	let account_id = create_account_id();
	let payl: [u8; 64] = [0; 64];
	let sig = sp_core::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(||{
		let mmr_root: H256 = create_mmr_with_one_account();
		let mut snapshot = EnclaveSnapshot::<AccountId32, Balance, WithdrawalLimit, AssetsLimit>{
			snapshot_number: 0,
    		merkle_root: mmr_root,
			withdrawals: bounded_vec![],
    		fees: bounded_vec![],

		};
		assert_ok!(
			OCEX::insert_enclave(
				Origin::root(),
				account_id.clone().into()
			)
		);
		assert_noop!(
			OCEX::submit_snapshot(
				Origin::signed(account_id.into()),
				snapshot,
				sig.clone().into()
			), 
			Error::<Test>::EnclaveSignatureVerificationFailed
		);

		assert_eq!(OCEX::ingress_messages().len(), 1);
	});
}

#[test]
fn test_submit_snapshot_bad_origin(){
	let payl: [u8; 64] = [0; 64];
	let sig = sp_core::sr25519::Signature::from_raw(payl);
	new_test_ext().execute_with(||{
		let mmr_root: H256 = create_mmr_with_one_account();
		let mut snapshot = EnclaveSnapshot::<AccountId32, Balance, WithdrawalLimit, AssetsLimit>{
			snapshot_number: 0,
    		merkle_root: mmr_root,
			withdrawals: bounded_vec![],
    		fees: bounded_vec![],

		};
		assert_noop!(
			OCEX::submit_snapshot(
				Origin::root(),
				snapshot.clone(),
				sig.clone().into()
			), 
			BadOrigin
		);

		assert_noop!(
			OCEX::submit_snapshot(
				Origin::root(), 
				snapshot, 
				sig.clone().into()
			), 
			BadOrigin
		);
	});
}

#[test]
fn test_submit_snapshot(){
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
	t.execute_with(||{
		let mmr_root: H256 = create_mmr_with_one_account();
		let mut snapshot = EnclaveSnapshot::<AccountId32, Balance, WithdrawalLimit, AssetsLimit>{
			snapshot_number: 0,
    		merkle_root: mmr_root,
			withdrawals: bounded_vec![],
    		fees: bounded_vec![],

		};
		assert_ok!(
			OCEX::insert_enclave(
				Origin::root(),
				account_id.clone().into()
			)
		);
		let bytes = snapshot.encode();
		let signature = public_key.sign(KEY_TYPE, &bytes).unwrap();
		
		assert_ok!(
			OCEX::submit_snapshot(
				Origin::signed(account_id.into()),
				snapshot,
				signature.clone().into()
			),
		);
		assert_eq!(Withdrawals::<Test>::contains_key(0), true);
		assert_eq!(FeesCollected::<Test>::contains_key(0), true);
		assert_eq!(Snapshots::<Test>::contains_key(0), true);
		assert_eq!(SnapshotNonce::<Test>::get().unwrap(), 1);
	})
}

#[test]
fn test_register_enclave(){
	let account_id = create_account_id(); 
	let ias_report = vec![19,19,2,7,255,128,6,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,7,0,0,0,0,0,0,0,7,0,0,0,0,0,0,0,143,217,40,231,107,115,187,6,31,209,73,221,16,254,236,124,82,125,255,243,103,55,60,72,2,212,150,41,217,255,51,13,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,95,183,141,57,75,101,149,246,85,227,219,71,14,143,143,79,2,209,127,165,117,206,185,73,81,228,1,225,150,116,242,38,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,173,228,47,97,157,17,22,141,70,202,241,106,128,71,84,214,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,20,173,193,15,23,193,76,218,39,74,232,62,213,215,194,80];
	new_test_ext().execute_with(||{
		assert_ok!(
			OCEX::register_enclave(
				Origin::signed(account_id.clone()),
				ias_report
			)
		);

		assert_last_event::<Test>(crate::Event::EnclaveRegistered(account_id).into())
	});
}

#[test]
fn test_register_enclave_empty_report(){
	let account_id = create_account_id(); 
	let ias_report = vec![];
	new_test_ext().execute_with(||{
		// TODO: Discuss this test, ideally this should fail I guess 
		assert_ok!(
			OCEX::register_enclave(
				Origin::signed(account_id),
				ias_report
			)
		);
	});
}

#[test]
fn test_reigster_enclave_bad_origin(){
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::register_enclave(
				Origin::root(), 
				vec![]
			), 
			BadOrigin
		);

		assert_noop!(
			OCEX::register_enclave(
				Origin::none(),
				vec![]
			),
			BadOrigin
		);
	});
}

#[test]
fn test_withdrawal_invalid_withdrawal_index(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::withdraw(
				Origin::signed(account_id.clone().into()),
				1,
				1
			), 
			Error::<Test>::InvalidWithdrawalIndex
		);
	});
}

#[test]
fn test_withdrawal_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		assert_noop!(
			OCEX::withdraw(
				Origin::root(),
				1,
				1
			), 
			BadOrigin
		);

		assert_noop!(
			OCEX::withdraw(
				Origin::none(),
				1,
				1
			), 
			BadOrigin
		);
	});
}

#[test]
fn test_shutdown(){
	let account_id = create_account_id();
	new_test_ext().execute_with(||{
		assert_ok!(
			OCEX::shutdown(
				Origin::root()
			)
		);

		let ingress_message: IngressMessages<AccountId32, BalanceOf::<Test>> = IngressMessages::Shutdown;
		assert_eq!(OCEX::ingress_messages()[0], ingress_message);
		assert_eq!(ExchangeState::<Test>::get(), false);
	});
}

#[test]
fn test_shutdown_bad_origin(){
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::shutdown(
				Origin::signed(account_id.into())
			), 
			BadOrigin
		);

		assert_noop!(
			OCEX::shutdown(
				Origin::none()
			), 
			BadOrigin
		);
	});
}


fn mint_into_account(account_id: AccountId32){
	Balances::deposit_creating(&account_id, 100000000000000);
}

fn create_asset_and_credit(asset_id: u128, account_id: AccountId32){
	assert_ok!(
		Assets::create(
			Origin::signed(account_id.clone().into()),
			asset_id.into(),
			account_id.clone().into(),
			100_u128		
		)
	);
}

fn create_account_id() -> AccountId32{
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

	return account_id;
}

fn create_public_key() -> sp_core::sr25519::Public {
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


fn create_mmr_with_one_account() -> H256{
	let account_id = create_account_id();
	let mut snapshot: BTreeMap<AccountId, AccountInfo<AccountId, Balance, ProxyLimit>> = Default::default();
	assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
	let account_info = Accounts::<Test>::get::<AccountId32>(account_id.clone().into()).unwrap();
	snapshot.insert(account_id.clone().into(), account_info.clone().into());
	let mmr_root = calculate_mmr_root(&mut snapshot.values()).unwrap();
	// panic!("{:?}", mmr_root.0);
	mmr_root
}

#[derive(Eq, PartialEq, Clone, Debug, Default)]
pub struct MergeAccountInfo(pub [u8; 32]);
impl Merge for MergeAccountInfo {
    type Item = MergeAccountInfo;
    fn merge(lhs: &Self::Item, rhs: &Self::Item) -> Result<Self::Item, ckb_merkle_mountain_range::Error> {
        let mut bytes = Vec::<u8>::with_capacity(64);
        lhs.0.map(|byte| bytes.push(byte));
        rhs.0.map(|byte| bytes.push(byte));
        Ok(MergeAccountInfo(sp_core::blake2_256(&bytes)))
    }
}

/// Calculates the MMR root for a given vector of accounts
pub fn calculate_mmr_root(
    accounts: &mut Values<AccountId, AccountInfo<AccountId, Balance, ProxyLimit>>,
) -> anyhow::Result<H256> {
    let store = MemStore::default();
    let mut mmr = MMR::<_, MergeAccountInfo, _>::new(0, &store);
    accounts.by_ref().for_each(|value| {
        let bytes = value.encode();
        if let Err(err) = mmr.push(MergeAccountInfo(sp_core::blake2_256(&bytes))) {
            log::error!(target: "mmr", "Unable to push account into MMR calculator: {:?}", err);
        }
    });

    match mmr.get_root() {
        Ok(root) => Ok(H256::from(root.0)),
        Err(err) => Err(anyhow::Error::msg(format!("unable to calculate MMR root: {:?}", err))),
    }
}