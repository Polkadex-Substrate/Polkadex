// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

use frame_support::{assert_noop, assert_ok};
use parity_scale_codec::Decode;
use sp_core::{H160, U256};
use sp_runtime::{
	BoundedBTreeSet, BoundedVec, DispatchError, DispatchError::BadOrigin, TokenError,
};

use crate::{
	mock,
	mock::{new_test_ext, Test, *},
	pallet::*,
};

const ASSET_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
const RECIPIENT_ADDRESS: &str = "0x0Edd7B63bDc5D0E88F7FDd8A38F802450f458fBA";
const UNIT_BALANCE: u128 = 1_000_000_000_000;

#[test]
pub fn test_create_asset_will_successfully_create_asset() {
	let (asset_address, recipient, chain_id) = create_asset_data();

	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(recipient),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
	});
}

#[test]
pub fn test_create_asset_with_already_existed_asset_will_return_already_registered() {
	let (asset_address, recipient, chain_id) = create_asset_data();

	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(recipient),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			asset_id.into(),
			recipient,
			100
		));
		assert_eq!(Assets::balance(asset_id, recipient), 100);

		// Re-register Asset
		assert_noop!(
			AssetHandler::create_asset(
				RuntimeOrigin::signed(recipient),
				chain_id,
				asset_address,
				PrecisionType::LowPrecision(1000000)
			),
			chainbridge::Error::<Test>::ResourceAlreadyRegistered
		);
	});
}

#[test]
pub fn test_mint_asset_with_invalid_resource_id() {
	let (asset_address, relayer, recipient, _, chain_id, account) = mint_asset_data();
	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(account),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let mut rid_malicious = rid.clone();
		rid_malicious[17] = rid[17].saturating_sub(1);
		let asset_id = AssetHandler::convert_asset_id(rid);
		let asset_id_malicious = AssetHandler::convert_asset_id(rid_malicious);

		assert_eq!(asset_id, asset_id_malicious);
		// Add new Relayer and verify storage
		assert_ok!(ChainBridge::add_relayer(RuntimeOrigin::signed(account), relayer));
		assert!(ChainBridge::relayers(relayer));

		// Mint Asset using Relayer account and verify storage
		assert_noop!(
			AssetHandler::mint_asset(
				RuntimeOrigin::signed(ChainBridge::account_id()),
				recipient.to_vec(),
				100000000,
				rid_malicious
			),
			chainbridge::Error::<Test>::ResourceDoesNotExist
		);
	});
}

#[test]
pub fn test_register_asset_twice_create_error() {
	let (asset_address, _, _, _, chain_id, account) = mint_asset_data();
	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(account),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		assert_noop!(
			AssetHandler::create_asset(
				RuntimeOrigin::signed(account),
				chain_id,
				asset_address,
				PrecisionType::LowPrecision(1000000)
			),
			chainbridge::Error::<Test>::ResourceAlreadyRegistered
		);
	});
}

#[test]
pub fn test_mint_asset_with_not_registered_asset_will_return_unknown_asset_error() {
	let (asset_address, relayer, recipient, recipient_account, chain_id, account) =
		mint_asset_data();

	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let mut asset_id = AssetHandler::convert_asset_id(rid);
		asset_id = asset_id + 100000000;
		// Add new Relayer and verify storage
		assert_ok!(ChainBridge::add_relayer(RuntimeOrigin::signed(account), relayer));
		assert!(ChainBridge::relayers(relayer));

		// Assert `mint_asset` will fail
		assert_noop!(
			AssetHandler::mint_asset(
				RuntimeOrigin::signed(ChainBridge::account_id()),
				recipient.to_vec(),
				100000000000000000000000000000000000000,
				rid
			),
			TokenError::UnknownAsset
		);
		// Assert balance of not created asset is 0
		assert_eq!(Assets::balance(asset_id, recipient_account), 0);
	});
}

#[test]
pub fn test_mint_asset_with_existed_asset_will_successfully_increase_balance() {
	let (asset_address, relayer, recipient, recipient_account, chain_id, account) =
		mint_asset_data();

	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(account),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		// Add new Relayer and verify storage
		assert_ok!(ChainBridge::add_relayer(RuntimeOrigin::signed(account), relayer));
		assert!(ChainBridge::relayers(relayer));

		// Mint Asset using Relayer account and verify storage
		assert_ok!(AssetHandler::mint_asset(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			recipient.to_vec(),
			100,
			rid
		));
		assert_eq!(Assets::balance(asset_id, recipient_account), 100000000);
	});
}

#[test]
pub fn test_mint_asset_called_by_not_relayer_will_return_minter_must_be_relayer_error() {
	let (asset_address, _relayer, recipient, recipient_account, chain_id, account) =
		mint_asset_data();

	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(account),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);

		assert_noop!(
			AssetHandler::mint_asset(RuntimeOrigin::signed(account), recipient.to_vec(), 100, rid),
			Error::<Test>::MinterIsNotValid
		);
		assert_eq!(Assets::balance(asset_id, recipient_account), 0);
	});
}

#[test]
pub fn test_block_delay_with_multiple_withdrawals() {
	let (asset_address, recipient, sender, chain_id) = withdraw_data();
	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_ok!(ChainBridge::allowlist_chain(RuntimeOrigin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(1),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			asset_id.into(),
			sender,
			400000000
		));
		System::set_block_number(100);
		assert_ok!(AssetHandler::set_block_delay(RuntimeOrigin::signed(1), 10));
		assert_ok!(AssetHandler::withdraw(
			RuntimeOrigin::signed(sender),
			chain_id,
			asset_address,
			100000000,
			recipient
		));
		assert_ok!(AssetHandler::set_block_delay(RuntimeOrigin::signed(1), 5));
		assert_ok!(AssetHandler::withdraw(
			RuntimeOrigin::signed(sender),
			chain_id,
			asset_address,
			200000000,
			recipient
		));
		let first_withdrawal = AssetHandler::get_pending_withdrawls(110);
		assert!(!first_withdrawal.is_empty());
		let second_withdrawal = AssetHandler::get_pending_withdrawls(105);
		assert!(!second_withdrawal.is_empty());
		mock::run_to_block(105);
		assert_eq!(
			ChainBridge::bridge_events(),
			vec![chainbridge::BridgeEvent::FungibleTransfer(
				chain_id,
				1,
				rid,
				U256::from(200),
				recipient.0.to_vec()
			)]
		);
		mock::run_to_block(110);
		assert_eq!(
			ChainBridge::bridge_events(),
			vec![
				chainbridge::BridgeEvent::FungibleTransfer(
					chain_id,
					1,
					rid,
					U256::from(200),
					recipient.0.to_vec()
				),
				chainbridge::BridgeEvent::FungibleTransfer(
					chain_id,
					2,
					rid,
					U256::from(100),
					recipient.0.to_vec()
				)
			]
		);
		assert_eq!(Assets::balance(asset_id, sender), 100000000);
	});
}

#[test]
pub fn test_withdraw_successfully() {
	let (asset_address, recipient, sender, chain_id) = withdraw_data();

	new_test_ext().execute_with(|| {
		// Setup
		allowlist_token(asset_address);
		assert_ok!(ChainBridge::allowlist_chain(RuntimeOrigin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(1),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			asset_id.into(),
			sender,
			2000000
		));
		System::set_block_number(100);
		assert_ok!(AssetHandler::set_block_delay(RuntimeOrigin::signed(1), 10));
		assert_ok!(AssetHandler::withdraw(
			RuntimeOrigin::signed(sender),
			chain_id,
			asset_address,
			1000000,
			recipient
		));
		let a = AssetHandler::get_pending_withdrawls(110);
		assert!(!a.is_empty());
		mock::run_to_block(110);
		assert_eq!(
			ChainBridge::bridge_events(),
			vec![chainbridge::BridgeEvent::FungibleTransfer(
				chain_id,
				1,
				rid,
				U256::from(1),
				recipient.0.to_vec()
			)]
		);
		assert_eq!(Assets::balance(asset_id, sender), 1000000);
	});
}

#[test]
pub fn test_allowlist_and_blacklist_token() {
	new_test_ext().execute_with(|| {
		let new_token = H160::random();
		assert_ok!(AssetHandler::allowlist_token(RuntimeOrigin::signed(1), new_token));
		let allowlisted_tokens = <AllowlistedToken<Test>>::get();
		assert!(allowlisted_tokens.contains(&new_token));
		assert_ok!(AssetHandler::remove_allowlisted_token(RuntimeOrigin::signed(1), new_token));
		let allowlisted_tokens = <AllowlistedToken<Test>>::get();
		assert!(!allowlisted_tokens.contains(&new_token));
	});
}

#[test]
pub fn test_allowlist_with_limit_reaching_returns_error() {
	new_test_ext().execute_with(|| {
		let mut allowlisted_assets: BoundedBTreeSet<H160, AllowlistedTokenLimit> =
			BoundedBTreeSet::new();
		for ele in 0..50 {
			assert_ok!(allowlisted_assets.try_insert(H160::from_low_u64_be(ele)));
		}
		assert_eq!(allowlisted_assets.len(), 50);
		<AllowlistedToken<Test>>::put(allowlisted_assets);
		let new_token = H160::random();
		assert_noop!(
			AssetHandler::allowlist_token(RuntimeOrigin::signed(1), new_token),
			Error::<Test>::AllowlistedTokenLimitReached
		);
	});
}

#[test]
pub fn test_withdraw_with_not_allowlisted_chain_will_return_chain_is_not_allowlisted_error() {
	let (asset_address, recipient, sender, chain_id) = withdraw_data();

	new_test_ext().execute_with(|| {
		allowlist_token(asset_address);
		assert_noop!(
			AssetHandler::withdraw(
				RuntimeOrigin::signed(sender),
				chain_id,
				asset_address,
				100,
				recipient
			),
			Error::<Test>::ChainIsNotAllowlisted
		);
	});
}

#[test]
pub fn test_withdraw_on_not_registered_asset_will_return_not_enough_balance_error() {
	let (asset_address, recipient, sender, chain_id) = withdraw_data();

	new_test_ext().execute_with(|| {
		// Setup
		allowlist_token(asset_address);
		assert_ok!(ChainBridge::allowlist_chain(RuntimeOrigin::signed(1), chain_id));

		assert_noop!(
			AssetHandler::withdraw(
				RuntimeOrigin::signed(sender),
				chain_id,
				asset_address,
				100,
				recipient
			),
			Error::<Test>::NotEnoughBalance
		);
	});
}

#[test]
pub fn test_withdraw_with_disabled_bridge_will_return_bridge_error() {
	let (asset_address, recipient, sender, chain_id) = withdraw_data();

	new_test_ext().execute_with(|| {
		// Setup
		allowlist_token(asset_address);
		assert_ok!(ChainBridge::allowlist_chain(RuntimeOrigin::signed(1), chain_id));
		<BridgeDeactivated<Test>>::put(true);
		assert!(<BridgeDeactivated<Test>>::get());
		assert_noop!(
			AssetHandler::withdraw(
				RuntimeOrigin::signed(sender),
				chain_id,
				asset_address,
				100,
				recipient
			),
			Error::<Test>::BridgeDeactivated
		);
	});
}

#[test]
pub fn test_withdraw_with_sender_not_enough_balance_will_return_not_enough_balance_error() {
	let (asset_address, recipient, sender, chain_id) = withdraw_data();

	new_test_ext().execute_with(|| {
		// Setup
		allowlist_token(asset_address);
		assert_ok!(ChainBridge::allowlist_chain(RuntimeOrigin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(1),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			asset_id.into(),
			sender,
			100
		));

		assert_noop!(
			AssetHandler::withdraw(
				RuntimeOrigin::signed(sender),
				chain_id,
				asset_address,
				1000,
				recipient
			),
			Error::<Test>::NotEnoughBalance
		);
	});
}

#[test]
pub fn test_withdraw_with_sender_not_enough_balance_for_fee_will_return_insufficient_balance_error()
{
	let (asset_address, recipient, sender, chain_id) = withdraw_data();

	new_test_ext().execute_with(|| {
		// Setup
		allowlist_token(asset_address);
		assert_ok!(ChainBridge::allowlist_chain(RuntimeOrigin::signed(1), chain_id));
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(1),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);
		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			asset_id.into(),
			sender,
			1000
		));

		assert_ok!(AssetHandler::withdraw(
			RuntimeOrigin::signed(sender),
			chain_id,
			asset_address,
			100,
			recipient
		));

		assert_ok!(AssetHandler::update_fee(RuntimeOrigin::signed(1), chain_id, 10, 100));
		assert_noop!(
			AssetHandler::withdraw(
				RuntimeOrigin::signed(sender),
				chain_id,
				asset_address,
				10,
				recipient
			),
			pallet_balances::Error::<Test>::InsufficientBalance
		);
	});
}

fn allowlist_token(token: H160) {
	let mut allowlisted_token = <AllowlistedToken<Test>>::get();
	allowlisted_token.try_insert(token).unwrap();
	<AllowlistedToken<Test>>::put(allowlisted_token);
}

#[test]
pub fn test_update_fee_successfully() {
	let chain_id = 2;

	new_test_ext().execute_with(|| {
		assert_ok!(AssetHandler::update_fee(RuntimeOrigin::signed(1), chain_id, 10, 100));
		assert_eq!(AssetHandler::get_bridge_fee(chain_id), (10, 100));
	});
}

#[test]
pub fn test_set_bridge_status() {
	new_test_ext().execute_with(|| {
		let new_bridge_status = true;
		assert_ok!(AssetHandler::set_bridge_status(RuntimeOrigin::signed(1), new_bridge_status));
		assert_eq!(<BridgeDeactivated<Test>>::get(), true);
	});
}

#[test]
pub fn test_set_block_delay() {
	new_test_ext().execute_with(|| {
		let no_of_blocks = 40;
		assert_ok!(AssetHandler::set_block_delay(RuntimeOrigin::signed(1), no_of_blocks));
		assert_eq!(<WithdrawalExecutionBlockDiff<Test>>::get(), no_of_blocks);
	});
}

#[test]
pub fn test_account_balances() {
	let (asset_address, _recipient, _sender, chain_id) = withdraw_data();
	new_test_ext().execute_with(|| {
		let balances_vec = AssetHandler::account_balances(vec![1], 1_u64);
		assert_eq!(balances_vec.len(), 1);
		assert_eq!(balances_vec[0], 0_u128);

		// Mint some amount
		assert_ok!(AssetHandler::create_asset(
			RuntimeOrigin::signed(1_u64),
			chain_id,
			asset_address,
			PrecisionType::LowPrecision(1000000)
		));
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		let asset_id = AssetHandler::convert_asset_id(rid);

		// Check Empty balances using helper function
		let balances_vec = AssetHandler::account_balances(vec![asset_id], 1_u64);
		assert_eq!(balances_vec.len(), 1);
		assert_eq!(balances_vec[0], 0_u128);

		assert_ok!(Assets::mint(
			RuntimeOrigin::signed(ChainBridge::account_id()),
			asset_id.into(),
			1_u64,
			100
		));
		assert_eq!(Assets::balance(asset_id, 1_u64), 100);

		// Check Balances now using helper function for different assset_ids
		let balances_vec = AssetHandler::account_balances(vec![1, asset_id], 1_u64);
		assert_eq!(balances_vec.len(), 2);
		assert_eq!(balances_vec[1], 100_u128);
	});
}

#[test]
pub fn test_convert_amount_for_native_chain() {
	let (asset_address, _recipient, _sender, chain_id) = withdraw_data();
	new_test_ext().execute_with(|| {
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		<AssetPrecision<Test>>::insert(rid, PrecisionType::HighPrecision(10000));
		assert_eq!(AssetHandler::convert_amount_for_native_chain(rid, 100000000), Some(10000));
		<AssetPrecision<Test>>::insert(rid, PrecisionType::LowPrecision(10000));
		assert_eq!(AssetHandler::convert_amount_for_native_chain(rid, 100), Some(1000000));
		<AssetPrecision<Test>>::insert(rid, PrecisionType::SamePrecision);
		assert_eq!(AssetHandler::convert_amount_for_native_chain(rid, 100), Some(100));
	});
}

#[test]
pub fn test_convert_amount_for_foreign_chain() {
	let (asset_address, _recipient, _sender, chain_id) = withdraw_data();
	new_test_ext().execute_with(|| {
		let rid = chainbridge::derive_resource_id(chain_id, &asset_address.0);
		<AssetPrecision<Test>>::insert(rid, PrecisionType::HighPrecision(10000));
		assert_eq!(
			AssetHandler::convert_amount_for_foreign_chain(rid, 100),
			Some(U256::from(1000000))
		);
		<AssetPrecision<Test>>::insert(rid, PrecisionType::LowPrecision(10000));
		assert_eq!(
			AssetHandler::convert_amount_for_foreign_chain(rid, 1000000),
			Some(U256::from(100))
		);
		<AssetPrecision<Test>>::insert(rid, PrecisionType::SamePrecision);
		assert_eq!(AssetHandler::convert_amount_for_foreign_chain(rid, 100), Some(U256::from(100)));
	});
}

#[test]
pub fn test_mint_thea_asset_with_not_registered_asset_will_return_ok() {
	let recipient = create_recipient_account();
	let asset_id = create_thea_asset_id(0, 5);

	new_test_ext().execute_with(|| {
		assert_ok!(AssetHandler::mint_thea_asset(asset_id, recipient, 1_000_000_000_000_0_u128));
	})
}

#[test]
pub fn test_burn_thea_asset_with_not_registered_asset_will_return_asset_not_registered_error() {
	let user = create_recipient_account();
	let non_register_asset_id = 2;
	new_test_ext().execute_with(|| {
		assert_noop!(
			AssetHandler::burn_thea_asset(non_register_asset_id, user, 100_u128),
			pallet_assets::Error::<Test>::Unknown
		);
	})
}

fn create_asset_data() -> (H160, u64, u8) {
	let asset_address: H160 = ASSET_ADDRESS.parse().unwrap();
	let recipient = create_recipient_account();
	let chain_id = 1;

	(asset_address, recipient, chain_id)
}

fn mint_asset_data() -> (H160, u64, [u8; 32], u64, u8, u64) {
	let asset_address: H160 = ASSET_ADDRESS.parse().unwrap();
	let relayer = 1u64;
	let recipient = [1u8; 32];
	let recipeint_account = create_recipient_account();
	let chain_id = 1;
	let account = 0u64;

	(asset_address, relayer, recipient, recipeint_account, chain_id, account)
}

fn withdraw_data() -> (H160, H160, u64, u8) {
	let asset_address: H160 = ASSET_ADDRESS.parse().unwrap();
	let recipient: H160 = RECIPIENT_ADDRESS.parse().unwrap();
	let sender = 2u64;
	let chain_id = 2;

	(asset_address, recipient, sender, chain_id)
}

fn create_recipient_account() -> u64 {
	let recipient = [1u8; 32];

	<Test as frame_system::Config>::AccountId::decode(&mut &recipient[..]).unwrap()
}

fn create_thea_asset_id(network_id: u8, identifier_length: u8) -> u128 {
	let asset_address: H160 = ASSET_ADDRESS.parse().unwrap();
	let mut derived_asset_id = vec![];

	derived_asset_id.push(network_id);
	derived_asset_id.push(identifier_length);
	derived_asset_id.extend(&asset_address.to_fixed_bytes()[0..identifier_length as usize]);

	// Hash the resulting vector with Keccak256 Hashing Algorithm and retrieve first 16 bytes
	let derived_asset_id_hash = &sp_io::hashing::keccak_256(derived_asset_id.as_ref())[0..16];
	// Derive u128 from resulting bytes
	let mut temp = [0u8; 16];

	temp.copy_from_slice(derived_asset_id_hash);

	u128::from_le_bytes(temp)
}
