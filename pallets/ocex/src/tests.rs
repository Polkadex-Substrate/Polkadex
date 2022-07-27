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

//! Tests for pallet-example-basic.

use crate::*;
use frame_support::{
	parameter_types,
	traits::{ConstU128, ConstU64},
	PalletId,
	assert_noop, assert_ok,
};
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
	TokenError
};
use crate::mock::*;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

#[test]
fn test_register_main_account(){
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), false);
		assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.into()), true);
	});
}

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
fn test_add_proxy_account(){
	let account_id = create_account_id(); 

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_ok!(OCEX::add_proxy_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
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
				account_id.into()
			)
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