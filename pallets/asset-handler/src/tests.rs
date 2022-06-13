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
use frame_support::{assert_noop, assert_ok};
use sp_core::{H160, H256, U256};
use sp_runtime::DispatchError::Token;
use sp_runtime::TokenError;
use sp_runtime::traits::{BadOrigin, BlockNumberProvider};
use crate::mock::*;
use super::*;
use crate::mock::{new_test_ext, Test, PDEX};

use crate::pallet::*;

#[test]
pub fn test_asset_creator() {
	let asset_address: H160 = "0x0Edd7B63bDc5D0E88F7FDd8A38F802450f458fBC".parse().unwrap();
	new_test_ext().execute_with(|| {
		assert_ok!(AssetHandler::create_asset(Origin::signed(1), 1, asset_address));
		let rid = chainbridge::derive_resource_id(1, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(Origin::signed(ChainBridge::account_id()), asset_id, 1, 100));
		assert_eq!(Assets::balance(asset_id,1),100);

		// Re-register Asset
		assert_noop!(AssetHandler::create_asset(Origin::signed(1), 1, asset_address), pallet_assets::Error::<Test>::InUse);
	});
}

#[test]
pub fn test_mint_asset() {
	let asset_address: H160 = "0x0Edd7B63bDc5D0E88F7FDd8A38F802450f458fBC".parse().unwrap();
	let relayer = 1u64;
	let recipient = 2u64;
	let chain_id = 1;
	new_test_ext().execute_with(|| {
		assert_ok!(AssetHandler::create_asset(Origin::signed(1), chain_id, asset_address));
		let rid = chainbridge::derive_resource_id(1, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);

		// Add new Relayer and verify storage
		assert_ok!(ChainBridge::add_relayer(Origin::signed(1), relayer));
		assert!(ChainBridge::relayers(relayer));

		// Mint Asset using Relayer account and verify storage
		assert_ok!(AssetHandler::mint_asset(Origin::signed(relayer), recipient, 100, rid));
		assert_eq!(Assets::balance(asset_id,recipient),100);
	});

	/*
	Check Errors
	  * Asset is not registered
	  * Not called by relayer

	*/

	// Asset is not registered
	new_test_ext().execute_with(|| {
		// Add new Relayer and verify storage
		let rid = chainbridge::derive_resource_id(1, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(ChainBridge::add_relayer(Origin::signed(1), relayer));
		assert!(ChainBridge::relayers(relayer));
		assert_noop!(AssetHandler::mint_asset(Origin::signed(relayer), recipient, 100, rid), TokenError::UnknownAsset);
		assert_eq!(Assets::balance(asset_id,recipient),0);
	});

    // Not called by relayer
	new_test_ext().execute_with(|| {
		assert_ok!(AssetHandler::create_asset(Origin::signed(1), chain_id, asset_address));
		let rid = chainbridge::derive_resource_id(1, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_noop!(AssetHandler::mint_asset(Origin::signed(relayer), recipient, 100, rid), Error::<Test>::MinterMustBeRelayer);
		assert_eq!(Assets::balance(asset_id,recipient),0);
	});
}

#[test]
pub fn test_withdraw() {
	let asset_address: H160 = "0x0Edd7B63bDc5D0E88F7FDd8A38F802450f458fBC".parse().unwrap();
	let recipient: H160 = "0x0Edd7B63bDc5D0E88F7FDd8A38F802450f458fBA".parse().unwrap();
	let sender = 2u64;
	let chain_id = 2;
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(ChainBridge::whitelist_chain(Origin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(Origin::signed(1), chain_id, asset_address));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(Origin::signed(ChainBridge::account_id()), asset_id, sender, 1000));
		assert_ok!(AssetHandler::withdraw(Origin::signed(sender), chain_id, asset_address, 100, recipient));

		assert_eq!(ChainBridge::bridge_events(), vec![chainbridge::BridgeEvent::
		FungibleTransfer(chain_id, 1, rid,
						 U256::from(100000000), recipient.0.to_vec())]);
	});

	/*
	Check Errors
	  * Asset is not registered.
	  * Chain is not whitelisted.
	  * Sender doesnt have enough balance.
	  * Sender doesnt have enough native asset balance for fee.
	*/

	// Chain is not whitelisted.
	new_test_ext().execute_with(|| {
		assert_noop!(AssetHandler::withdraw(Origin::signed(sender), chain_id, asset_address, 100, recipient), Error::<Test>::ChainIsNotWhitelisted);
	});

	// Asset is not registered.
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(ChainBridge::whitelist_chain(Origin::signed(1), chain_id));

		assert_noop!(AssetHandler::withdraw(Origin::signed(sender), chain_id, asset_address, 100, recipient), Error::<Test>::NotEnoughBalance);
	});

	// Sender doesnt have enough balance.
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(ChainBridge::whitelist_chain(Origin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(Origin::signed(1), chain_id, asset_address));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(Origin::signed(ChainBridge::account_id()), asset_id, sender, 100));
		assert_noop!(AssetHandler::withdraw(Origin::signed(sender), chain_id, asset_address, 1000, recipient), Error::<Test>::NotEnoughBalance);
	});

	// Sender doesnt have enough native asset balance for fee.
	new_test_ext().execute_with(|| {
		// Setup
		assert_ok!(ChainBridge::whitelist_chain(Origin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(Origin::signed(1), chain_id, asset_address));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(Origin::signed(ChainBridge::account_id()), asset_id, sender, 1000));
		assert_ok!(AssetHandler::withdraw(Origin::signed(sender), chain_id, asset_address, 100, recipient));
		assert_ok!(AssetHandler::update_fee(Origin::signed(1), chain_id, 10, 100));

		assert_noop!(AssetHandler::withdraw(Origin::signed(sender), chain_id, asset_address, 10, recipient), pallet_balances::Error::<Test>::InsufficientBalance);
	});
}

#[test]
pub fn test_update_fee() {
	let chain_id = 2;

	new_test_ext().execute_with(|| {
        assert_ok!(AssetHandler::update_fee(Origin::signed(1), chain_id, 10, 100));
		assert_eq!(AssetHandler::get_bridge_fee(chain_id), (10, 100));
	});
}


