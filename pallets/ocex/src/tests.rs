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

//! Tests for pallet-ocex.

use crate::{storage::store_trie_root, *};
use frame_support::{assert_noop, assert_ok};
use polkadex_primitives::{assets::AssetId, withdrawal::Withdrawal, Signature, UNIT_BALANCE};
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use sp_std::collections::btree_map::BTreeMap;
use std::str::FromStr;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use crate::mock::*;
use frame_support::traits::fungibles::Inspect as InspectAsset;
use frame_support::traits::fungibles::Mutate as MutateAsset;
use frame_support::{testing_prelude::bounded_vec, BoundedVec};
use frame_system::EventRecord;
use parity_scale_codec::Decode;
use polkadex_primitives::ocex::AccountInfo;
use polkadex_primitives::{ingress::IngressMessages, AccountId, AssetsLimit};
use rust_decimal::Decimal;
use sp_core::{
	bounded::BoundedBTreeSet,
	offchain::{testing::TestOffchainExt, OffchainDbExt, OffchainWorkerExt},
	ByteArray, Pair, H256,
};
use sp_keystore::{testing::MemoryKeystore, Keystore};
use sp_runtime::{AccountId32, DispatchError::BadOrigin, SaturatedConversion, TokenError};
use sp_std::default::Default;

pub fn register_offchain_ext(ext: &mut sp_io::TestExternalities) {
	let (offchain, _offchain_state) = TestOffchainExt::with_offchain_db(ext.offchain_db());
	ext.register_extension(OffchainDbExt::new(offchain.clone()));
	ext.register_extension(OffchainWorkerExt::new(offchain));
}

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

fn assert_last_event<T: Config>(generic_event: <T as Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

//Alice Account
pub const ALICE_MAIN_ACCOUNT_RAW_ID: [u8; 32] = [6u8; 32];
pub const ALICE_PROXY_ACCOUNT_RAW_ID: [u8; 32] = [7u8; 32];

fn get_alice_accounts() -> (AccountId32, AccountId32) {
	(AccountId::new(ALICE_MAIN_ACCOUNT_RAW_ID), AccountId::new(ALICE_PROXY_ACCOUNT_RAW_ID))
}

#[test]
fn test_ocex_submit_snapshot() {
	let auth1 = sp_core::sr25519::Pair::generate().0;
	let auth2 = sp_core::sr25519::Pair::generate().0;
	let auth3 = sp_core::sr25519::Pair::generate().0;
	let authorities = vec![
		AuthorityId::from(auth1.public()),
		AuthorityId::from(auth2.public()),
		AuthorityId::from(auth3.public()),
	];

	let snapshot = SnapshotSummary {
		validator_set_id: 0,
		snapshot_id: 114,
		state_hash: H256::random(),
		state_change_id: 1104,
		last_processed_blk: 1103,
		withdrawals: vec![],
		egress_messages: vec![],
		trader_metrics: None,
	};

	let signature1 = auth1.sign(&snapshot.encode());

	let signature2 = auth2.sign(&snapshot.encode());

	new_test_ext().execute_with(|| {
		<Authorities<Test>>::insert(0, ValidatorSet::new(authorities, 0));
		<SnapshotNonce<Test>>::put(113);
		OCEX::validate_snapshot(
			&snapshot,
			&vec![(0, signature1.clone().into()), (1, signature2.clone().into())],
		)
		.unwrap();
		assert_ok!(OCEX::submit_snapshot(
			RuntimeOrigin::none(),
			snapshot,
			vec![(0, signature1.into()), (1, signature2.into())]
		));
		assert_eq!(<SnapshotNonce<Test>>::get(), 114);
	});
}

#[test]
// check if balance is added to new account
fn test_add_balance_new_account() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_id = create_account_id();
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone()),
			account_id.clone()
		));
		let asset_id = AssetId::Polkadex;
		let amount = 1000000;
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		let result = add_balance(&mut state, &account_id, asset_id, amount.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&asset_id).unwrap(), &amount.into());
		// test get_balance()
		state.commit().unwrap();
		drop(state);
		store_trie_root(root);
		let from_fn = OCEX::get_balance(account_id.clone(), asset_id).unwrap();
		assert_eq!(from_fn, amount.into());
		// test get_ob_recover_state()
		let rs = OCEX::get_ob_recover_state().unwrap();
		assert!(!rs.1.is_empty());
		assert!(!rs.2.is_empty());
		// account present
		assert!(rs.1.get(&account_id).is_some_and(|v| !v.is_empty() && v[0] == account_id));
		// balance present and correct
		let expected: Decimal = amount.into();
		assert_eq!(
			rs.2.get(&AccountAsset { main: account_id, asset: asset_id }).unwrap(),
			&expected
		);
	});
}

#[test]
// check if balance is added to existing account with balance
fn test_add_balance_existing_account_with_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_id = create_account_id();
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone()),
			account_id.clone()
		));
		let asset_id = AssetId::Polkadex;
		let amount = 1000000;
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		let result = add_balance(&mut state, &account_id, asset_id, amount.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&asset_id).unwrap(), &amount.into());

		//add more balance
		let amount2 = 2000000;
		let result = add_balance(&mut state, &account_id, asset_id, amount2.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&asset_id).unwrap(), &(amount + amount2).into());
		// test get_balance()
		state.commit().unwrap();
		drop(state);
		store_trie_root(root);
		let from_fn = OCEX::get_balance(account_id.clone(), asset_id).unwrap();
		assert_eq!(from_fn, (amount + amount2).into());
		// test get_ob_recover_state()
		let rs = OCEX::get_ob_recover_state().unwrap();
		assert!(!rs.1.is_empty());
		assert!(!rs.2.is_empty());
		// account present
		assert!(rs.1.get(&account_id).is_some_and(|v| !v.is_empty() && v[0] == account_id));
		// balance present and correct
		let expected: Decimal = (amount + amount2).into();
		assert_eq!(
			rs.2.get(&AccountAsset { main: account_id, asset: asset_id }).unwrap(),
			&expected
		);
		// conversion test
		let created = ObRecoveryState {
			snapshot_id: rs.0,
			account_ids: rs.1.clone(),
			balances: rs.2.clone(),
			last_processed_block_number: rs.3,
			state_change_id: rs.4,
			worker_nonce: rs.5,
		};
		let c_encoded = created.encode();
		let encoded = rs.encode();
		assert_eq!(c_encoded, encoded);
		let decoded = ObRecoveryState::decode(&mut encoded.as_ref()).unwrap();
		assert_eq!(decoded.account_ids, rs.1);
		assert_eq!(decoded.balances, rs.2);
	});
}

#[test]
fn test_two_assets() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_bytes = [1u8; 32];
		let pablo_main = AccountId::from(account_bytes);

		let account_bytes = [2u8; 32];
		let coinalpha = AccountId::from(account_bytes);

		let account_id = pablo_main.clone();
		let asset1 = AssetId::Asset(123);
		let amount1 = Decimal::from_str("0.05").unwrap();

		let asset2 = AssetId::Asset(456);
		let amount2 = Decimal::from_str("0.1").unwrap();
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		add_balance(&mut state, &account_id, asset1, amount1.into()).unwrap();
		add_balance(&mut state, &account_id, asset2, amount2.into()).unwrap();
		let asset123 = AssetId::Asset(123);
		let amount123 = Decimal::from_str("25.0").unwrap();

		let asset456 = AssetId::Asset(456);
		let amount456 = Decimal::from_str("10.0").unwrap();
		// works
		sub_balance(&mut state, &account_id, asset1, Decimal::from_str("0.01").unwrap().into())
			.unwrap();
		add_balance(&mut state, &coinalpha, asset123, amount123.into()).unwrap();
		add_balance(&mut state, &coinalpha, asset456, amount456.into()).unwrap();
		let root = state.commit().unwrap();
		store_trie_root(root);
		drop(state);
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		sub_balance(&mut state, &account_id, asset1, Decimal::from_str("0.01").unwrap().into())
			.unwrap();
		sub_balance(&mut state, &account_id, asset1, Decimal::from_str("0.01").unwrap().into())
			.unwrap();
	});
}

#[test]
// check if balance can be subtracted from a new account
fn test_sub_balance_new_account() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_id = create_account_id();
		let asset_id = AssetId::Polkadex;
		let amount = 1000000;
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		let result = sub_balance(&mut state, &account_id, asset_id, amount.into());
		match result {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, "Account not found in trie"),
		}
	});
}

#[test]
// check if balance can be subtracted from existing account
fn test_sub_balance_existing_account_with_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_id = create_account_id();
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone()),
			account_id.clone()
		));
		let asset_id = AssetId::Polkadex;
		let amount = 3000000;
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		let result = add_balance(&mut state, &account_id, asset_id, amount.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&asset_id).unwrap(), &amount.into());

		//sub balance
		let amount2 = 2000000;
		let result = sub_balance(&mut state, &account_id, asset_id, amount2.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&asset_id).unwrap(), &(amount - amount2).into());

		//sub balance till 0
		let amount3 = amount - amount2;
		let result = sub_balance(&mut state, &account_id, asset_id, amount3.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(amount - amount2 - amount3, 0);
		assert_eq!(account_info.get(&asset_id).unwrap(), &Decimal::from(0));
		// test get_balance()
		state.commit().unwrap();
		drop(state);
		store_trie_root(root);
		let from_fn = OCEX::get_balance(account_id.clone(), asset_id).unwrap();
		assert_eq!(from_fn, (amount - amount2 - amount3).into());
		// test get_ob_recover_state()
		let rs = OCEX::get_ob_recover_state().unwrap();
		assert!(!rs.1.is_empty());
		assert!(!rs.2.is_empty());
		// account present
		assert!(rs.1.get(&account_id).is_some_and(|v| !v.is_empty() && v[0] == account_id));
		// balance present and correct
		let expected: Decimal = (amount - amount2 - amount3).into();
		assert_eq!(
			rs.2.get(&AccountAsset { main: account_id, asset: asset_id }).unwrap(),
			&expected
		);
	});
}

#[test]
fn test_trie_update() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		assert!(state.is_empty());

		state.insert(b"a".to_vec(), b"1".to_vec());
		state.insert(b"b".to_vec(), b"2".to_vec());
		state.insert(b"c".to_vec(), b"3".to_vec());
		assert!(!state.is_empty());
		let root = state.commit().unwrap(); // This should flush everything to db.
		crate::storage::store_trie_root(root);
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);

		assert_eq!(state.get(&b"a".to_vec()).unwrap().unwrap(), b"1");
		assert_eq!(state.get(&b"b".to_vec()).unwrap().unwrap(), b"2");
		assert_eq!(state.get(&b"c".to_vec()).unwrap().unwrap(), b"3");

		state.insert(b"d".to_vec(), b"4".to_vec()); // This will not be in DB, as neither root() or commit() is called

		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let state = crate::storage::get_state_trie(&mut trie_state, &mut root);
		assert_eq!(state.get(b"a").unwrap().unwrap(), b"1");
		assert_eq!(state.get(b"b").unwrap().unwrap(), b"2");
		assert_eq!(state.get(b"c").unwrap().unwrap(), b"3");
		assert_eq!(state.get(b"d").unwrap(), None);
	})
}

#[test]
// check if balance can be subtracted from existing account
fn test_balance_update_depost_first_then_trade() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_id = create_account_id();
		let amount = 20;
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);

		let result = add_balance(
			&mut state,
			&&Decode::decode(&mut &account_id.encode()[..]).unwrap(),
			AssetId::Polkadex,
			amount.into(),
		);
		assert_eq!(result, Ok(()));

		//add balance for another asset
		let amount2 = Decimal::from_f64_retain(4.2).unwrap();
		let result = add_balance(&mut state, &account_id, AssetId::Asset(1), amount2.into());
		assert_eq!(result, Ok(()));

		//sub balance till 0
		let amount3 = Decimal::from_f64_retain(2.0).unwrap();
		let result = sub_balance(&mut state, &account_id, AssetId::Polkadex, amount3.into());
		assert_eq!(result, Ok(()));
	});
}

#[test]
// check if more than available balance can be subtracted from existing account
fn test_sub_more_than_available_balance_from_existing_account_with_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let account_id = create_account_id();
		let asset_id = AssetId::Polkadex;
		let amount = 3000000;
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		let result = add_balance(&mut state, &account_id, asset_id, amount.into());
		assert_eq!(result, Ok(()));
		let encoded = state.get(&account_id.to_raw_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&asset_id).unwrap(), &amount.into());

		//sub balance
		let amount2 = 4000000;
		let result = sub_balance(&mut state, &account_id, asset_id, amount2.into());
		match result {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, "NotEnoughBalance"),
		}
	});
}

#[test]
// check if balance is added to new account
fn test_trade_between_two_accounts_without_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);
		let config = get_trading_pair_config();
		let amount = Decimal::from_str("20").unwrap();
		let price = Decimal::from_str("2").unwrap();
		let trade = create_trade_between_alice_and_bob(price, amount);
		let (maker_fees, taker_fees) =
			OCEX::get_fee_structure(&trade.maker.user, &trade.taker.user).unwrap();
		let result = OCEX::process_trade(&mut state, &trade, config, maker_fees, taker_fees);
		match result {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, "NotEnoughBalance"),
		}
	});
}

#[test]
// check if balance is added to new account
fn test_trade_between_two_accounts_with_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);

		// add balance to alice
		let alice_account_id = get_alice_key_pair().public();
		let initial_asset_1_alice_has = 40;
		let _initial_pdex_alice_has = 0;
		assert_ok!(add_balance(
			&mut state,
			&alice_account_id.into(),
			AssetId::Asset(1),
			initial_asset_1_alice_has.into()
		));

		//add balance to bob
		let bob_account_id = get_bob_key_pair().public();
		let initial_pdex_bob_has = 20;
		let initial_asset_1_bob_has = 0;
		assert_ok!(add_balance(
			&mut state,
			&bob_account_id.into(),
			AssetId::Polkadex,
			initial_pdex_bob_has.into()
		));

		//market PDEX-1
		let config = get_trading_pair_config();
		let amount = Decimal::from_str("20").unwrap();
		let price = Decimal::from_str("2").unwrap();

		//alice bought 20 PDEX from bob for a price of 2 PDEX per Asset(1)
		// total trade value = 20 PDEX and 40 Asset(1)
		//so alice should have 20 PDEX and bob should have 20 less PDEX
		//also, alice should have 40 less Asset(1) and bob should have 40 more Asset(1)
		let trade = create_trade_between_alice_and_bob(price, amount);
		let (maker_fees, taker_fees) =
			OCEX::get_fee_structure(&trade.maker.user, &trade.taker.user).unwrap();
		let result = OCEX::process_trade(&mut state, &trade, config, maker_fees, taker_fees);
		assert_ok!(result);

		//check has 20 pdex now
		let encoded = state.get(&alice_account_id.0.to_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(account_info.get(&AssetId::Polkadex).unwrap(), &20.into());

		//check if bob has 20 less pdex
		let encoded = state.get(&bob_account_id.0.to_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(
			account_info.get(&AssetId::Polkadex).unwrap(),
			&(initial_pdex_bob_has - 20).into()
		);

		//check if bob has 40 more asset_1
		let encoded = state.get(&bob_account_id.0.to_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(
			account_info.get(&AssetId::Asset(1)).unwrap(),
			&(initial_asset_1_bob_has + 40).into()
		);

		//check if alice has 40 less asset_1
		let encoded = state.get(&alice_account_id.0.to_vec()).unwrap().unwrap();
		let account_info: BTreeMap<AssetId, Decimal> = BTreeMap::decode(&mut &encoded[..]).unwrap();
		assert_eq!(
			account_info.get(&AssetId::Asset(1)).unwrap(),
			&(initial_asset_1_alice_has - 40).into()
		);
	});
}

#[test]
// check if balance is added to new account
fn test_trade_between_two_accounts_insuffient_bidder_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);

		// add balance to alice
		let alice_account_id = get_alice_key_pair().public();
		assert_ok!(add_balance(&mut state, &alice_account_id.into(), AssetId::Asset(1), 39.into()));

		//add balance to bob
		let bob_account_id = get_bob_key_pair().public();
		assert_ok!(add_balance(&mut state, &bob_account_id.into(), AssetId::Polkadex, 20.into()));

		//market PDEX-1
		let config = get_trading_pair_config();
		let amount = Decimal::from_str("20").unwrap();
		let price = Decimal::from_str("2").unwrap();

		//alice bought 20 PDEX from bob for a price of 2 PDEX per Asset(1)
		let trade = create_trade_between_alice_and_bob(price, amount);
		let (maker_fees, taker_fees) =
			OCEX::get_fee_structure(&trade.maker.user, &trade.taker.user).unwrap();
		let result = OCEX::process_trade(&mut state, &trade, config, maker_fees, taker_fees);
		match result {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, "NotEnoughBalance"),
		}
	});
}

#[test]
// check if balance is added to new account
fn test_trade_between_two_accounts_insuffient_asker_balance() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);

		// add balance to alice
		let alice_account_id = get_alice_key_pair().public();
		assert_ok!(add_balance(&mut state, &alice_account_id.into(), AssetId::Asset(1), 40.into()));

		//add balance to bob
		let bob_account_id = get_bob_key_pair().public();
		assert_ok!(add_balance(&mut state, &bob_account_id.into(), AssetId::Polkadex, 19.into()));

		//market PDEX-1
		let config = get_trading_pair_config();
		let amount = Decimal::from_str("20").unwrap();
		let price = Decimal::from_str("2").unwrap();

		//alice bought 20 PDEX from bob for a price of 2 PDEX per Asset(1)
		let trade = create_trade_between_alice_and_bob(price, amount);
		let (maker_fees, taker_fees) =
			OCEX::get_fee_structure(&trade.maker.user, &trade.taker.user).unwrap();
		let result = OCEX::process_trade(&mut state, &trade, config, maker_fees, taker_fees);
		match result {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, "NotEnoughBalance"),
		}
	});
}

#[test]
// check if balance is added to new account
fn test_trade_between_two_accounts_invalid_signature() {
	let mut ext = new_test_ext();
	ext.persist_offchain_overlay();
	register_offchain_ext(&mut ext);
	ext.execute_with(|| {
		let mut root = crate::storage::load_trie_root();
		let mut trie_state = crate::storage::State;
		let mut state = OffchainState::load(&mut trie_state, &mut root);

		// add balance to alice
		let alice_account_id = get_alice_key_pair().public();
		assert_ok!(add_balance(&mut state, &alice_account_id.into(), AssetId::Asset(1), 40.into()));

		//add balance to bob
		let bob_account_id = get_bob_key_pair().public();
		assert_ok!(add_balance(&mut state, &bob_account_id.into(), AssetId::Polkadex, 20.into()));

		//market PDEX-1
		let config = get_trading_pair_config();
		let amount = Decimal::from_str("20").unwrap();
		let price = Decimal::from_str("2").unwrap();

		//alice bought 20 PDEX from bob for a price of 2 PDEX per Asset(1)
		let mut trade = create_trade_between_alice_and_bob(price, amount);
		//swap alice and bob's signature
		trade.maker.signature = trade.taker.signature.clone();
		let (maker_fees, taker_fees) =
			OCEX::get_fee_structure(&trade.maker.user, &trade.taker.user).unwrap();
		let result = OCEX::process_trade(&mut state, &trade, config, maker_fees, taker_fees);
		match result {
			Ok(_) => assert!(false),
			Err(e) => assert_eq!(e, "InvalidTrade"),
		}
	});
}

#[test]
fn test_register_main_account() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), false);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
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
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[1], event);
	});
}

#[test]
fn test_register_main_account_main_account_already_exists() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.clone().into()), true);
		assert_noop!(
			OCEX::register_main_account(
				RuntimeOrigin::signed(account_id.clone().into()),
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
			OCEX::register_main_account(RuntimeOrigin::root(), account_id.clone().into()),
			BadOrigin
		);
		assert_noop!(
			OCEX::register_main_account(RuntimeOrigin::none(), account_id.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_add_proxy_account_main_account_not_found() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				account_id.into()
			),
			Error::<Test>::MainAccountNotFound
		);
	});
}

#[test]
fn test_add_proxy_account_exchange_state_not_operational() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				account_id.into()
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_add_proxy_account_proxy_limit_exceeded() {
	let account_id = create_account_id();
	let proxy_account1 = create_proxy_account("1");
	let proxy_account2 = create_proxy_account("2");
	let proxy_account3 = create_proxy_account("3");
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			proxy_account1.clone().into()
		));
		assert_ok!(OCEX::add_proxy_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			proxy_account2.clone().into()
		));
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				proxy_account3.clone().into()
			),
			Error::<Test>::ProxyLimitExceeded
		);
	})
}

#[test]
fn test_add_proxy_account_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::add_proxy_account(RuntimeOrigin::root(), account_id.clone().into()),
			BadOrigin
		);

		assert_noop!(
			OCEX::add_proxy_account(RuntimeOrigin::none(), account_id.clone().into()),
			BadOrigin
		);
	});
}

#[test]
fn test_add_proxy_account() {
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone().into()
		));
		assert_noop!(
			OCEX::add_proxy_account(
				RuntimeOrigin::signed(account_id.clone().into()),
				account_id.clone().into()
			),
			Error::<Test>::ProxyAlreadyRegistered
		);
		assert_last_event::<Test>(
			crate::Event::MainAccountRegistered {
				main: account_id.clone(),
				proxy: account_id.clone(),
			}
			.into(),
		);

		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 2);
	});
}

#[test]
fn test_register_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Polkadex,
				10001_u128.into(),
				100_u128.into(),
				10001_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::BothAssetsCannotBeSame
		);
	});
}

#[test]
fn test_register_trading_pair_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Polkadex,
				10001_u128.into(),
				100_u128.into(),
				10001_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_register_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::none(),
				AssetId::Polkadex,
				AssetId::Polkadex,
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
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Polkadex,
				AssetId::Polkadex,
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
fn test_register_trading_pair_value_zero() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				0_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigCannotBeZero
		);
	});
}

#[test]
fn test_register_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));

		assert_eq!(
			TradingPairs::<Test>::contains_key(AssetId::Asset(10), AssetId::Asset(20)),
			true
		);
		assert_eq!(
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20))
				.unwrap()
				.operational_status,
			true
		);
		assert_last_event::<Test>(
			crate::Event::TradingPairRegistered {
				base: AssetId::Asset(10),
				quote: AssetId::Asset(20),
			}
			.into(),
		);
		let trading_pair =
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		let event: IngressMessages<AccountId32> = IngressMessages::OpenTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[1], event);
	});
}

#[test]
fn test_register_trading_pair_amount_overflow() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_update_trading_pair_amount_overflow() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			10000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				DEPOSIT_MAX + 1,
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1,
				10_u128.into()
			),
			Error::<Test>::AmountOverflow
		);
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				100_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				DEPOSIT_MAX + 1
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_register_trading_pair_trading_pair_already_registered() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));

		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
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
				RuntimeOrigin::root(),
				AssetId::Asset(20),
				AssetId::Asset(10),
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
fn test_update_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_ok!(OCEX::update_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));

		assert_last_event::<Test>(
			crate::Event::TradingPairUpdated {
				base: AssetId::Asset(10),
				quote: AssetId::Asset(20),
			}
			.into(),
		);
		let trading_pair =
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		let event: IngressMessages<AccountId32> = IngressMessages::UpdateTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[3], event);
	});
}

#[test]
fn test_update_trading_pair_with_less_than_min_volume() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Polkadex,
			AssetId::Asset(1),
			10001_u128.into(),
			100_u128.into(),
			10001_u128.into(),
			100_u128.into(),
			100_u128.into(),
			10_u128.into()
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Polkadex,
			AssetId::Asset(1),
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Asset(1),
				10000_u128.into(),
				100_u128.into(),
				10000_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigUnderflow
		);
	});
}

#[test]
fn test_update_trading_pair_trading_pair_not_registered() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::TradingPairNotRegistered
		);
	});
}

#[test]
fn test_update_trading_pair_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_update_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::none(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			BadOrigin
		);
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into()
			),
			BadOrigin
		);
	});
}

#[test]
fn test_register_trading_pair_volume_too_low() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::register_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Polkadex,
				AssetId::Asset(1),
				10000_u128.into(),
				100_u128.into(),
				10000_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigUnderflow
		);
	});
}

#[test]
fn test_update_trading_pair_value_zero() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));

		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				0_u128.into(),
				100_u128.into(),
				1_u128.into(),
				100_u128.into(),
				100_u128.into(),
				10_u128.into(),
			),
			Error::<Test>::TradingPairConfigCannotBeZero
		);
	});
}

#[test]
fn test_deposit_unknown_asset() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		let asset_id = AssetId::Asset(10);
		allowlist_token(asset_id);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				asset_id,
				100_u128.into()
			),
			TokenError::UnknownAsset
		);
	});
}

#[test]
fn test_deposit_exchange_not_operational() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Asset(10),
				100_u128.into()
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_deposit_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));

		assert_noop!(
			OCEX::deposit(RuntimeOrigin::root(), AssetId::Asset(10), 100_u128.into()),
			BadOrigin
		);

		assert_noop!(
			OCEX::deposit(RuntimeOrigin::none(), AssetId::Asset(10), 100_u128.into()),
			BadOrigin
		);
	});
}

#[test]
fn test_deposit_account_not_registered() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Asset(10));
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Asset(10),
				100_u128.into()
			),
			Error::<Test>::AccountNotRegistered
		);
	});
}

#[test]
fn test_deposit() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		mint_into_account(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			10000000000000000000000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		allowlist_token(AssetId::Polkadex);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_ok!(OCEX::deposit(
			RuntimeOrigin::signed(account_id.clone().into()),
			AssetId::Polkadex,
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
				asset: AssetId::Polkadex,
				amount: 100_u128,
			}
			.into(),
		);
		let event: IngressMessages<AccountId32> =
			IngressMessages::Deposit(account_id, AssetId::Polkadex, Decimal::new(10, 11));
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[2], event);
	});
}

#[test]
fn test_deposit_large_value() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		mint_into_account_large(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			1_000_000_000_000_000_000_000_000_000_000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		allowlist_token(AssetId::Polkadex);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Polkadex,
				1_000_000_000_000_000_000_000_000_0000
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_deposit_assets_overflow() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		mint_into_account_large(account_id.clone());
		// Balances before deposit
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			1_000_000_000_000_000_000_000_000_000_000
		);
		assert_eq!(<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()), 0);
		allowlist_token(AssetId::Polkadex);
		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(account_id.clone().into()),
			account_id.clone()
		));
		assert_ok!(OCEX::deposit(
			RuntimeOrigin::signed(account_id.clone().into()),
			AssetId::Polkadex,
			1_000_000_000_000_000_000_000_000_000
		));
		let large_value: Decimal = Decimal::MAX;
		mint_into_account_large(account_id.clone());
		// Directly setting the storage value, found it very difficult to manually fill it up
		TotalAssets::<Test>::insert(
			AssetId::Polkadex,
			large_value.saturating_sub(Decimal::from_u128(1).unwrap()),
		);

		assert_noop!(
			OCEX::deposit(
				RuntimeOrigin::signed(account_id.clone().into()),
				AssetId::Polkadex,
				10_u128.pow(20)
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn test_open_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::BothAssetsCannotBeSame
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_open_trading_pair_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_open_trading_pair_trading_pair_not_found() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(20)),
			Error::<Test>::TradingPairNotFound
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_open_trading_pair_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::open_trading_pair(RuntimeOrigin::none(), AssetId::Asset(10), AssetId::Asset(20)),
			BadOrigin
		);

		assert_noop!(
			OCEX::open_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Asset(10),
				AssetId::Asset(20)
			),
			BadOrigin
		);
	});
}

#[test]
fn test_open_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::open_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));
		assert_eq!(
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20))
				.unwrap()
				.operational_status,
			true
		);
		let trading_pair = OCEX::trading_pairs(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		assert_last_event::<Test>(
			crate::Event::OpenTradingPair { pair: trading_pair.clone() }.into(),
		);
		let event: IngressMessages<AccountId32> = IngressMessages::OpenTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[1], event);
	})
}

#[test]
fn test_close_trading_pair_both_assets_cannot_be_same() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::BothAssetsCannotBeSame
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_close_trading_exchange_not_operational() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(10)),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn test_close_trading_trading_pair_not_found() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::root(), AssetId::Asset(10), AssetId::Asset(20)),
			Error::<Test>::TradingPairNotFound
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 1);
	});
}

#[test]
fn test_close_trading_trading_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::close_trading_pair(RuntimeOrigin::none(), AssetId::Asset(10), AssetId::Asset(20)),
			BadOrigin
		);

		assert_noop!(
			OCEX::close_trading_pair(
				RuntimeOrigin::signed(account_id.into()),
				AssetId::Asset(10),
				AssetId::Asset(20)
			),
			BadOrigin
		);
	});
}

#[test]
fn test_close_trading_pair() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_ok!(OCEX::close_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20)
		));
		assert_eq!(
			TradingPairs::<Test>::get(AssetId::Asset(10), AssetId::Asset(20))
				.unwrap()
				.operational_status,
			false
		);
		let trading_pair = OCEX::trading_pairs(AssetId::Asset(10), AssetId::Asset(20)).unwrap();
		assert_last_event::<Test>(
			crate::Event::ShutdownTradingPair { pair: trading_pair.clone() }.into(),
		);
		let event: IngressMessages<AccountId32> = IngressMessages::CloseTradingPair(trading_pair);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk)[2], event);
	})
}

#[test]
fn test_update_trading_pair_with_closed_operational_status() {
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_ok!(OCEX::register_trading_pair(
			RuntimeOrigin::root(),
			AssetId::Asset(10),
			AssetId::Asset(20),
			1_0000_0000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_000_000_000_000_000_u128.into(),
			1_000_000_u128.into(),
			1_0000_0000_u128.into(),
		));
		assert_noop!(
			OCEX::update_trading_pair(
				RuntimeOrigin::root(),
				AssetId::Asset(10),
				AssetId::Asset(20),
				1_0000_0000_u128.into(),
				1_000_000_000_000_000_u128.into(),
				1_000_000_u128.into(),
				1_000_000_000_000_000_u128.into(),
				1_000_0000_u128.into(),
				1_0000_000_u128.into(),
			),
			Error::<Test>::TradingPairIsNotClosed
		);
	})
}

#[test]
fn collect_fees_unexpected_behaviour() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		assert_ok!(OCEX::collect_fees(RuntimeOrigin::root(), 100, account_id.clone().into()));

		assert_last_event::<Test>(
			crate::Event::FeesClaims { beneficiary: account_id, snapshot_id: 100 }.into(),
		);
	});
}

#[test]
fn test_collect_fees_decimal_overflow() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		let max_fees = create_max_fees::<Test>();
		FeesCollected::<Test>::insert::<u64, BoundedVec<Fees, AssetsLimit>>(
			0,
			bounded_vec![max_fees],
		);
		assert_noop!(
			OCEX::collect_fees(RuntimeOrigin::root(), 0, account_id.into()),
			Error::<Test>::FeesNotCollectedFully
		);
	})
}

#[test]
fn collect_fees() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	let mut t = new_test_ext();
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());
		let initial_balance = 10_000_000_000 * UNIT_BALANCE;
		// Initial Balances
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance
		);

		let (mut snapshot, _public, signature) = get_dummy_snapshot(1);

		snapshot.withdrawals[0].fees = Decimal::from_f64(0.1).unwrap();

		assert_ok!(OCEX::submit_snapshot(
			RuntimeOrigin::none(),
			snapshot.clone(),
			vec![(0, signature.into())]
		));

		// Complete dispute period
		new_block();
		new_block();

		assert_ok!(OCEX::claim_withdraw(
			RuntimeOrigin::signed(account_id.clone().into()),
			1,
			account_id.clone()
		));

		// Balances after withdrawal
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance + UNIT_BALANCE
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance - UNIT_BALANCE
		);

		assert_ok!(OCEX::collect_fees(RuntimeOrigin::root(), 1, account_id.clone().into()));

		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance
				+ UNIT_BALANCE + snapshot.withdrawals[0]
				.fees
				.saturating_mul(Decimal::from(UNIT_BALANCE))
				.to_u128()
				.unwrap()
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance
				- UNIT_BALANCE - snapshot.withdrawals[0]
				.fees
				.saturating_mul(Decimal::from(UNIT_BALANCE))
				.to_u128()
				.unwrap()
		);
	});
}

#[test]
fn test_collect_fees_bad_origin() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::collect_fees(
				RuntimeOrigin::signed(account_id.clone()),
				100,
				account_id.clone().into()
			),
			BadOrigin
		);

		assert_noop!(
			OCEX::collect_fees(RuntimeOrigin::signed(account_id.clone()), 100, account_id.into()),
			BadOrigin
		);
	});
}

#[test]
fn withdrawal_when_exchange_not_operational() {
	let (alice_account_id, proxy_account_id) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_account_id.clone(),
				proxy_account_id,
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::ExchangeNotOperational
		);
	});
}

#[test]
fn withdrawal_when_token_not_allowlisted() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_main_account.clone(),
				alice_proxy_account,
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::TokenNotAllowlisted
		);
	});
}

#[test]
fn withdrawal_when_account_not_register() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Polkadex);
		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_main_account.clone(),
				alice_proxy_account,
				AssetId::Polkadex,
				100_u128.saturated_into(),
				true
			),
			Error::<Test>::AccountNotRegistered
		);
	});
}

#[test]
fn withdrawal_with_overflow_amount() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Polkadex);

		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(alice_main_account.clone().into()),
			alice_proxy_account.clone().into()
		));

		assert_noop!(
			OCEX::withdrawal_from_orderbook(
				alice_main_account.clone(),
				alice_proxy_account,
				AssetId::Polkadex,
				(WITHDRAWAL_MAX + 1).saturated_into(),
				true
			),
			Error::<Test>::AmountOverflow
		);
	});
}

#[test]
fn withdrawal() {
	let (alice_main_account, alice_proxy_account) = get_alice_accounts();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		allowlist_token(AssetId::Polkadex);

		assert_ok!(OCEX::register_main_account(
			RuntimeOrigin::signed(alice_main_account.clone().into()),
			alice_proxy_account.clone().into()
		));

		assert_ok!(OCEX::withdrawal_from_orderbook(
			alice_main_account.clone(),
			alice_proxy_account.clone(),
			AssetId::Polkadex,
			100_u128.saturated_into(),
			true
		));
		let blk = frame_system::Pallet::<Test>::current_block_number();
		//assert ingress message
		assert_eq!(
			OCEX::ingress_messages(blk)[2],
			IngressMessages::DirectWithdrawal(
				alice_proxy_account,
				AssetId::Polkadex,
				Decimal::new(100, 12),
				true,
			)
		);

		//assert event
		assert_last_event::<Test>(
			crate::Event::WithdrawFromOrderbook(alice_main_account, AssetId::Polkadex, 100_u128)
				.into(),
		);
	});
}

// P.S. This was to apply a DDOS attack and see the response in the mock environment
#[ignore]
#[test]
fn collect_fees_ddos() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		// TODO! Discuss if this is expected behaviour, if not then could this be a potential DDOS?
		for x in 0..10000000 {
			assert_ok!(OCEX::collect_fees(RuntimeOrigin::root(), x, account_id.clone().into()));
		}
	});
}

#[test]
fn test_submit_snapshot_snapshot_nonce_error() {
	new_test_ext().execute_with(|| {
		let (mut snapshot, _public, _) = get_dummy_snapshot(0);
		snapshot.snapshot_id = 2;
		// Wrong nonce
		assert_noop!(
			OCEX::validate_snapshot(&snapshot, &Vec::new()),
			InvalidTransaction::Custom(10)
		);
		let blk = frame_system::Pallet::<Test>::current_block_number();
		assert_eq!(OCEX::ingress_messages(blk).len(), 0);
	});
}

fn get_dummy_snapshot(
	withdrawals_len: usize,
) -> (SnapshotSummary<AccountId32>, sp_core::sr25519::Public, sp_core::sr25519::Signature) {
	let main = create_account_id();

	let mut withdrawals = vec![];
	for _ in 0..withdrawals_len {
		withdrawals.push(Withdrawal {
			main_account: main.clone(),
			amount: Decimal::one(),
			asset: AssetId::Polkadex,
			fees: Default::default(),
			stid: 0,
		})
	}

	let pair = sp_core::sr25519::Pair::generate().0;
	let snapshot = SnapshotSummary {
		validator_set_id: 0,
		snapshot_id: 1,
		state_hash: Default::default(),
		state_change_id: 1,
		last_processed_blk: 1,
		withdrawals,
		egress_messages: vec![],
		trader_metrics: None,
	};

	let signature = pair.sign(&snapshot.encode());

	(snapshot, pair.public(), signature)
}

#[test]
fn test_submit_snapshot_bad_origin() {
	new_test_ext().execute_with(|| {
		let (snapshot, _public, signature) = get_dummy_snapshot(1);
		assert_noop!(
			OCEX::validate_snapshot(&snapshot, &vec![(0, signature.into())]),
			InvalidTransaction::Custom(12)
		);
	});
}

#[test]
fn test_submit_snapshot() {
	let _account_id = create_account_id();
	let mut t = new_test_ext();
	t.execute_with(|| {
		let (mut snapshot, _public, _signature) = get_dummy_snapshot(1);
		snapshot.withdrawals[0].fees = Decimal::from_f64(1.0).unwrap();
		let mut withdrawal_map = BTreeMap::new();
		for withdrawal in &snapshot.withdrawals {
			match withdrawal_map.get_mut(&withdrawal.main_account) {
				None => {
					withdrawal_map
						.insert(withdrawal.main_account.clone(), vec![withdrawal.clone()]);
				},
				Some(list) => {
					list.push(withdrawal.clone());
				},
			}
		}
		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot.clone(), Vec::new()));

		assert_eq!(Withdrawals::<Test>::contains_key(1), true);
		assert_eq!(Withdrawals::<Test>::get(1), withdrawal_map.clone());
		assert_eq!(FeesCollected::<Test>::contains_key(1), true);
		assert_eq!(Snapshots::<Test>::contains_key(1), true);
		assert_eq!(Snapshots::<Test>::get(1).unwrap(), snapshot.clone());
		assert_eq!(SnapshotNonce::<Test>::get(), 1);
		let onchain_events =
			vec![polkadex_primitives::ocex::OnChainEvents::OrderbookWithdrawalProcessed(
				1,
				snapshot.withdrawals.clone(),
			)];
		assert_eq!(OnChainEvents::<Test>::get(), onchain_events);
		// Checking for redundant data inside snapshot
		assert_eq!(Snapshots::<Test>::get(1).unwrap().withdrawals, snapshot.withdrawals);
	})
}

fn new_block() {
	let number = frame_system::Pallet::<Test>::block_number() + 1;
	let hash = H256::repeat_byte(number as u8);

	frame_system::Pallet::<Test>::reset_events();
	frame_system::Pallet::<Test>::initialize(&number, &hash, &Default::default())
}

#[test]
fn test_withdrawal_invalid_withdrawal_index() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_noop!(
			OCEX::claim_withdraw(
				RuntimeOrigin::signed(account_id.clone().into()),
				1,
				account_id.clone()
			),
			Error::<Test>::InvalidWithdrawalIndex
		);
	});
}

#[test]
fn test_withdrawal() {
	let account_id = create_account_id();
	let custodian_account = OCEX::get_pallet_account();
	let mut t = new_test_ext();
	t.execute_with(|| {
		mint_into_account(account_id.clone());
		mint_into_account(custodian_account.clone());

		let initial_balance = 10_000_000_000 * UNIT_BALANCE;
		// Initial Balances
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance
		);

		let (snapshot, _public, _signature) = get_dummy_snapshot(1);

		assert_ok!(OCEX::submit_snapshot(RuntimeOrigin::none(), snapshot.clone(), Vec::new()));

		// Complete dispute period
		new_block();
		new_block();

		assert_ok!(OCEX::claim_withdraw(
			RuntimeOrigin::signed(account_id.clone().into()),
			1,
			account_id.clone()
		));
		// Balances after withdrawal
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(account_id.clone()),
			initial_balance + UNIT_BALANCE // Increased by 1
		);
		assert_eq!(
			<Test as Config>::NativeCurrency::free_balance(custodian_account.clone()),
			initial_balance - UNIT_BALANCE, // Dec
		);
		let withdrawal_claimed: polkadex_primitives::ocex::OnChainEvents<AccountId> =
			polkadex_primitives::ocex::OnChainEvents::OrderBookWithdrawalClaimed(
				1,
				account_id.clone().into(),
				bounded_vec![snapshot.withdrawals[0].clone()],
			);
		assert_eq!(OnChainEvents::<Test>::get()[1], withdrawal_claimed);
	});
}

use orderbook_primitives::{
	recovery::ObRecoveryState,
	types::{Order, OrderPayload, OrderSide, OrderStatus, OrderType, Trade},
	Fees, TraderMetricsMap, TradingPairMetrics, TradingPairMetricsMap,
};
use sp_runtime::traits::{BlockNumberProvider, One};

use orderbook_primitives::types::UserActionBatch;
use trie_db::TrieMut;

#[test]
fn test_withdrawal_bad_origin() {
	new_test_ext().execute_with(|| {
		let account_id = create_account_id();
		assert_noop!(OCEX::claim_withdraw(RuntimeOrigin::root(), 1, account_id.clone()), BadOrigin);

		assert_noop!(OCEX::claim_withdraw(RuntimeOrigin::none(), 1, account_id.clone()), BadOrigin);
	});
}

#[test]
pub fn test_allowlist_and_blacklist_token() {
	new_test_ext().execute_with(|| {
		let _account_id = create_account_id();
		let new_token = AssetId::Asset(1);
		assert_ok!(OCEX::allowlist_token(RuntimeOrigin::root(), new_token));
		let allowlisted_tokens = <AllowlistedToken<Test>>::get();
		assert!(allowlisted_tokens.contains(&new_token));
		assert_ok!(OCEX::remove_allowlisted_token(RuntimeOrigin::root(), new_token));
		let allowlisted_tokens = <AllowlistedToken<Test>>::get();
		assert!(!allowlisted_tokens.contains(&new_token));
	});
}

#[test]
pub fn test_allowlist_with_limit_reaching_returns_error() {
	new_test_ext().execute_with(|| {
		let _account_id = create_account_id();
		let mut allowlisted_assets: BoundedBTreeSet<AssetId, AllowlistedTokenLimit> =
			BoundedBTreeSet::new();
		for ele in 0..50 {
			assert_ok!(allowlisted_assets.try_insert(AssetId::Asset(ele)));
		}
		assert_eq!(allowlisted_assets.len(), 50);
		<AllowlistedToken<Test>>::put(allowlisted_assets);
		let new_token = AssetId::Asset(100);
		assert_noop!(
			OCEX::allowlist_token(RuntimeOrigin::root(), new_token),
			Error::<Test>::AllowlistedTokenLimitReached
		);
	});
}

use crate::{
	settlement::{add_balance, sub_balance},
	sr25519::AuthorityId,
	storage::OffchainState,
};
use polkadex_primitives::ingress::{EgressMessages, HandleBalance, HandleBalanceLimit};

#[test]
pub fn test_set_balances_when_bounded_vec_limits_out_of_bound() {
	let account_id = create_account_id();
	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), false));
		let mut vec_of_balances: Vec<HandleBalance<AccountId32>> = vec![];
		for _i in 0..1001 {
			vec_of_balances.push(HandleBalance {
				main_account: account_id.clone(),
				asset_id: AssetId::Polkadex,
				free: 100,
				reserve: 50,
			});
		}
		let bounded_vec_for_alice: Result<
			BoundedVec<HandleBalance<AccountId>, HandleBalanceLimit>,
			Vec<HandleBalance<AccountId32>>,
		> = BoundedVec::try_from(vec_of_balances);
		assert!(bounded_vec_for_alice.is_err());
	});
}

#[test]
fn test_remove_proxy_account_faulty_cases() {
	let (main, proxy) = get_alice_accounts();
	new_test_ext().execute_with(|| {
		// bad origin
		assert_noop!(OCEX::remove_proxy_account(RuntimeOrigin::root(), proxy.clone()), BadOrigin);
		assert_noop!(OCEX::remove_proxy_account(RuntimeOrigin::none(), proxy.clone()), BadOrigin);
		// exchange not operational
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main.clone()), proxy.clone(),),
			Error::<Test>::ExchangeNotOperational
		);
		// no main account registered
		<ExchangeState<Test>>::set(true);
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main.clone()), proxy.clone(),),
			Error::<Test>::MainAccountNotFound
		);
		// minimum one proxy required
		OCEX::register_main_account(RuntimeOrigin::signed(main.clone()), proxy.clone()).unwrap();
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main.clone()), proxy.clone(),),
			Error::<Test>::MinimumOneProxyRequired
		);
		// no proxy account found
		<Accounts<Test>>::mutate(&main, |account_info| {
			if let Some(a) = account_info {
				a.proxies.pop();
				a.proxies.try_push(main.clone()).unwrap();
				a.proxies.try_push(main.clone()).unwrap();
			} else {
				panic!("failed to mutate Accounts")
			}
		});
		assert_noop!(
			OCEX::remove_proxy_account(RuntimeOrigin::signed(main), proxy,),
			Error::<Test>::ProxyNotFound
		);
	})
}

#[test]
fn test_remove_proxy_account_proper_case() {
	let (main, proxy) = get_alice_accounts();
	new_test_ext().execute_with(|| {
		<ExchangeState<Test>>::set(true);
		OCEX::register_main_account(RuntimeOrigin::signed(main.clone()), proxy.clone()).unwrap();
		<Accounts<Test>>::mutate(&main, |account_info| {
			if let Some(a) = account_info {
				a.proxies.try_push(main.clone()).unwrap();
				a.proxies.try_push(main.clone()).unwrap();
			} else {
				panic!("failed to mutate Accounts")
			}
		});
		assert_ok!(OCEX::remove_proxy_account(RuntimeOrigin::signed(main), proxy));
	})
}

#[test]
fn test_set_snapshot_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		// bad origins
		assert_noop!(OCEX::set_snapshot(RuntimeOrigin::none(), 1), BadOrigin);
		assert_noop!(OCEX::set_snapshot(RuntimeOrigin::signed(a), 1), BadOrigin);
		assert_noop!(OCEX::set_snapshot(RuntimeOrigin::signed(b), 1), BadOrigin);
		// proper cases
		assert_ok!(OCEX::set_snapshot(RuntimeOrigin::root(), 1));
	})
}

#[test]
fn test_set_exchange_state_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		// bad origins
		assert_noop!(OCEX::set_exchange_state(RuntimeOrigin::none(), true), BadOrigin);
		assert_noop!(OCEX::set_exchange_state(RuntimeOrigin::signed(a), true), BadOrigin);
		assert_noop!(OCEX::set_exchange_state(RuntimeOrigin::signed(b), true), BadOrigin);
		// proper case
		assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
		let current = frame_system::Pallet::<Test>::current_block_number();
		assert!(<crate::IngressMessages<Test>>::get(current).len() == 1);
	})
}

#[test]
fn test_whitelist_orderbook_operator_full() {
	new_test_ext().execute_with(|| {
		let (a, b) = get_alice_accounts();
		let key = sp_core::ecdsa::Pair::generate().0.public();
		// bad origins
		assert_noop!(OCEX::whitelist_orderbook_operator(RuntimeOrigin::none(), key), BadOrigin);
		assert_noop!(
			OCEX::whitelist_orderbook_operator(RuntimeOrigin::signed(a.clone()), key),
			BadOrigin
		);
		assert_noop!(
			OCEX::whitelist_orderbook_operator(RuntimeOrigin::signed(b.clone()), key),
			BadOrigin
		);
		// proper case
		assert_ok!(OCEX::whitelist_orderbook_operator(RuntimeOrigin::root(), key));
		assert_eq!(<OrderbookOperatorPublicKey<Test>>::get().unwrap(), key);
	})
}

#[ignore]
#[test]
fn test_old_user_action_enum_payload_with_new_enum_returns_ok() {
	let payload = r#"{"actions":[{"BlockImport":4842070},{"BlockImport":4842071},{"BlockImport":4842072},{"Withdraw":{"signature":{"Sr25519":"1ce02504db86d6c40826737a0616248570274d6fc880d1294585da3663efb41a8cd7f66db1666edbf0037e193ddf9597ec567e875ccb84b1187bbe6e5d1b5c88"},"payload":{"asset_id":{"asset":"95930534000017180603917534864279132680"},"amount":"0.01","timestamp":1690900017685},"main":"5GLQUnNXayJGG6AZ6ht2MFigMHLKPWZjZqbko2tYQ7GJxi6A","proxy":"5GeYN9KaGkxEzaP2gpefqpCp18a9MEMosPCintz83CGRpKGa"}},{"BlockImport":4842073},{"BlockImport":4842074},{"BlockImport":4842075},{"BlockImport":4842076},{"BlockImport":4842077},{"BlockImport":4842078},{"Withdraw":{"signature":{"Sr25519":"b8a7bb383882379a5cb3796c1fb362a9efca5c224c60e2bb91bfed7a9f94bb620620e32dcecbc7e64011e3d3d073b1290e46b3cb97cf0b96c49ba5b0e9e1548f"},"payload":{"asset_id":{"asset":"123"},"amount":"10","timestamp":1690900085111},"main":"5GLFKUxSXTf8MDDKM1vqEFb5TuV1q642qpQT964mrmjeKz4w","proxy":"5ExtoLVQaef9758mibzLhaxK4GBk7qoysSWo7FKt2nrV26i8"}},{"BlockImport":4842079},{"BlockImport":4842080},{"BlockImport":4842081},{"BlockImport":4842082},{"Withdraw":{"signature":{"Sr25519":"4e589e61b18815abcc3fe50626e54844d1e2fd9bb0575fce8eabb5af1ba4b42fba060ad3067bef341e8d5973d932f30d9113c0abbbd65e96e2dd5cbaf94d4581"},"payload":{"asset_id":{"asset":"456"},"amount":"4","timestamp":1690900140296},"main":"5GLFKUxSXTf8MDDKM1vqEFb5TuV1q642qpQT964mrmjeKz4w","proxy":"5ExtoLVQaef9758mibzLhaxK4GBk7qoysSWo7FKt2nrV26i8"}},{"BlockImport":4842083},{"BlockImport":4842084},{"BlockImport":4842085},{"BlockImport":4842086},{"BlockImport":4842087},{"BlockImport":4842088},{"BlockImport":4842089},{"BlockImport":4842090},{"BlockImport":4842091},{"BlockImport":4842092},{"BlockImport":4842093},{"BlockImport":4842094},{"BlockImport":4842095},{"BlockImport":4842096},{"BlockImport":4842097},{"BlockImport":4842098},{"BlockImport":4842099},{"BlockImport":4842100},{"BlockImport":4842101}],"stid":74132,"snapshot_id":10147,"signature":"901dc6972f94d69f253b9ca5a83410a5bc729e5c30c68cba3e68ea4860ca73e447d06c41d3bad05aca4e031f0fa46b1f64fac70159cec68151fef534e48515de00"}"#;
	let _: UserActionBatch<AccountId> = serde_json::from_str(payload).unwrap();
}

#[test]
fn test_set_lmp_epoch_config_happy_path() {
	new_test_ext().execute_with(|| {
		let total_liquidity_mining_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
		let total_trading_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		// Register trading pair
		crete_base_and_quote_asset();
		register_trading_pair();
		let mut market_weightage = BTreeMap::new();
		market_weightage.insert(trading_pair.clone(), UNIT_BALANCE);
		let market_weightage: Option<BTreeMap<TradingPair, u128>> = Some(market_weightage);
		let mut min_fees_paid = BTreeMap::new();
		min_fees_paid.insert(trading_pair.clone(), UNIT_BALANCE);
		let min_fees_paid: Option<BTreeMap<TradingPair, u128>> = Some(min_fees_paid);
		let mut min_maker_volume = BTreeMap::new();
		min_maker_volume.insert(trading_pair, UNIT_BALANCE);
		let min_maker_volume: Option<BTreeMap<TradingPair, u128>> = Some(min_maker_volume);
		let max_accounts_rewarded: Option<u16> = Some(10);
		let claim_safety_period: Option<u32> = Some(10);
		assert_ok!(OCEX::set_lmp_epoch_config(
			RuntimeOrigin::root(),
			total_liquidity_mining_rewards,
			total_trading_rewards,
			market_weightage,
			min_fees_paid,
			min_maker_volume,
			max_accounts_rewarded,
			claim_safety_period
		));
	})
}

#[test]
fn test_set_lmp_epoch_config_invalid_market_weightage() {
	new_test_ext().execute_with(|| {
		let total_liquidity_mining_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
		let total_trading_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		// Register trading pair
		crete_base_and_quote_asset();
		register_trading_pair();
		let mut market_weightage = BTreeMap::new();
		market_weightage.insert(trading_pair.clone(), 10 * UNIT_BALANCE);
		let market_weightage: Option<BTreeMap<TradingPair, u128>> = Some(market_weightage);
		let mut min_fees_paid = BTreeMap::new();
		min_fees_paid.insert(trading_pair.clone(), 10 * UNIT_BALANCE);
		let min_fees_paid: Option<BTreeMap<TradingPair, u128>> = Some(min_fees_paid);
		let mut min_maker_volume = BTreeMap::new();
		min_maker_volume.insert(trading_pair, UNIT_BALANCE);
		let min_maker_volume: Option<BTreeMap<TradingPair, u128>> = Some(min_maker_volume);
		let max_accounts_rewarded: Option<u16> = Some(10);
		let claim_safety_period: Option<u32> = Some(10);
		assert_noop!(
			OCEX::set_lmp_epoch_config(
				RuntimeOrigin::root(),
				total_liquidity_mining_rewards,
				total_trading_rewards,
				market_weightage,
				min_fees_paid,
				min_maker_volume,
				max_accounts_rewarded,
				claim_safety_period
			),
			crate::pallet::Error::<Test>::InvalidMarketWeightage
		);
	})
}

#[test]
fn test_set_lmp_epoch_config_invalid_invalid_lmpconfig() {
	new_test_ext().execute_with(|| {
		let total_liquidity_mining_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
		let total_trading_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		// Register trading pair
		crete_base_and_quote_asset();
		register_trading_pair();
		let mut market_weightage = BTreeMap::new();
		market_weightage.insert(trading_pair.clone(), UNIT_BALANCE);
		let market_weightage: Option<BTreeMap<TradingPair, u128>> = Some(market_weightage);
		let mut min_fees_paid = BTreeMap::new();
		let diff_quote_asset = AssetId::Asset(2);
		let trading_pair = TradingPair { base: base_asset, quote: diff_quote_asset };
		min_fees_paid.insert(trading_pair.clone(), 10 * UNIT_BALANCE);
		let min_fees_paid: Option<BTreeMap<TradingPair, u128>> = Some(min_fees_paid);
		let mut min_maker_volume = BTreeMap::new();
		min_maker_volume.insert(trading_pair, UNIT_BALANCE);
		let min_maker_volume: Option<BTreeMap<TradingPair, u128>> = Some(min_maker_volume);
		let max_accounts_rewarded: Option<u16> = Some(10);
		let claim_safety_period: Option<u32> = Some(10);
		assert_noop!(
			OCEX::set_lmp_epoch_config(
				RuntimeOrigin::root(),
				total_liquidity_mining_rewards,
				total_trading_rewards,
				market_weightage,
				min_fees_paid,
				min_maker_volume,
				max_accounts_rewarded,
				claim_safety_period
			),
			crate::pallet::Error::<Test>::InvalidLMPConfig
		);
	})
}

#[test]
fn test_update_lmp_scores_happy_path() {
	new_test_ext().execute_with(|| {
		add_lmp_config();
		let total_score = Decimal::from(1000);
		let total_fee_paid = Decimal::from(1000);
		let trading_pair_metrics: TradingPairMetrics = (total_score, total_fee_paid);
		let trader = AccountId32::new([1; 32]);
		let trader_score = Decimal::from(100);
		let trader_fee_paid = Decimal::from(100);
		let mut trader_metrics: TraderMetricsMap<AccountId32> = BTreeMap::new();
		trader_metrics.insert(trader.clone(), (trader_score, trader_fee_paid));
		let mut trading_pair_metrics_map: TradingPairMetricsMap<AccountId32> = BTreeMap::new();
		trading_pair_metrics_map.insert(
			TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
			(trader_metrics, trading_pair_metrics),
		);
		assert_ok!(OCEX::update_lmp_scores(&trading_pair_metrics_map));
	})
}

#[test]
fn test_update_lmp_scores_no_lmp_config() {
	new_test_ext().execute_with(|| {
		let total_score = Decimal::from(1000);
		let total_fee_paid = Decimal::from(1000);
		let trading_pair_metrics: TradingPairMetrics = (total_score, total_fee_paid);
		let trader = AccountId32::new([1; 32]);
		let trader_score = Decimal::from(100);
		let trader_fee_paid = Decimal::from(100);
		let mut trader_metrics: TraderMetricsMap<AccountId32> = BTreeMap::new();
		trader_metrics.insert(trader.clone(), (trader_score, trader_fee_paid));
		let mut trading_pair_metrics_map: TradingPairMetricsMap<AccountId32> = BTreeMap::new();
		trading_pair_metrics_map.insert(
			TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
			(trader_metrics, trading_pair_metrics),
		);
		<LMPEpoch<Test>>::put(2);
		assert_noop!(
			OCEX::update_lmp_scores(&trading_pair_metrics_map),
			crate::pallet::Error::<Test>::LMPConfigNotFound
		);
	})
}

#[test]
fn test_do_claim_lmp_rewards_happy_path() {
	new_test_ext().execute_with(|| {
		add_lmp_config();
		update_lmp_score();
		let main_account = AccountId32::new([1; 32]);
		let epoch = 1;
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
		let reward_account =
			<mock::Test as pallet::Config>::LMPRewardsPalletId::get().into_account_truncating();
		Balances::mint_into(&reward_account, 300 * UNIT_BALANCE).unwrap();
		assert_ok!(OCEX::do_claim_lmp_rewards(main_account.clone(), epoch, trading_pair));
		assert_eq!(Balances::free_balance(&main_account), 200999999999900u128);
	})
}

#[test]
fn test_process_egress_msg_trading_fee() {
	new_test_ext().execute_with(|| {
		crete_base_and_quote_asset();
		let asset_id = 1;
		let asset = AssetId::Asset(asset_id);
		let pallet_account = OCEX::get_pallet_account();
		let pot_account = OCEX::get_pot_account();
		Balances::mint_into(&pallet_account, 100 * UNIT_BALANCE).unwrap();
		Balances::mint_into(&pot_account, 100 * UNIT_BALANCE).unwrap();
		Assets::mint_into(asset_id, &pallet_account, 200 * UNIT_BALANCE).unwrap();
		let trader_fee_paid = Decimal::from(100);
		let mut fee_map = BTreeMap::new();
		fee_map.insert(asset, trader_fee_paid);
		let message = EgressMessages::TradingFees(fee_map);
		assert_ok!(OCEX::process_egress_msg(&vec![message]));
		assert_eq!(Assets::balance(asset_id, &pot_account), 100 * UNIT_BALANCE);
	})
}

#[test]
fn test_process_remove_liquidity_result() {
	new_test_ext().execute_with(|| {
		crete_base_and_quote_asset();
		let asset_id = 1;
		let asset = AssetId::Asset(asset_id);
		let market = TradingPairConfig {
			base_asset: AssetId::Polkadex,
			quote_asset: asset,
			min_price: Default::default(),
			max_price: Default::default(),
			price_tick_size: Default::default(),
			min_qty: Default::default(),
			max_qty: Default::default(),
			qty_step_size: Default::default(),
			operational_status: true,
			base_asset_precision: 12,
			quote_asset_precision: 12,
		};
		let pool = AccountId32::new([3; 32]);
		let lp = AccountId32::new([4; 32]);
		let pallet_account = OCEX::get_pallet_account();
		let base_free = Decimal::from(1);
		let quote_free = Decimal::from(1);
		Balances::mint_into(&pallet_account, 200 * UNIT_BALANCE).unwrap();
		Balances::mint_into(&pool, 1 * UNIT_BALANCE).unwrap();
		Assets::mint_into(asset_id, &pool, 1 * UNIT_BALANCE).unwrap();
		Assets::mint_into(asset_id, &pallet_account, 200 * UNIT_BALANCE).unwrap();
		let message = EgressMessages::RemoveLiquidityResult(
			market,
			pool.clone(),
			lp.clone(),
			base_free,
			quote_free,
		);
		assert_ok!(OCEX::process_egress_msg(&vec![message]));
		// Check balance
		assert_eq!(Balances::free_balance(&lp), 1 * UNIT_BALANCE);
		assert_eq!(Assets::balance(asset_id, &lp), 1 * UNIT_BALANCE);
	})
}

#[test]
fn test_price_oracle() {
	new_test_ext().execute_with(|| {
		let mut old_price_map = <PriceOracle<Test>>::get();
		let base_asset = AssetId::Polkadex;
		let quote_asset = AssetId::Asset(1);
		let avg_price = Decimal::from(100);
		let tick = Decimal::from(1);
		old_price_map.insert((base_asset, quote_asset), (avg_price, tick));
		<PriceOracle<Test>>::put(old_price_map);
		let mut new_price_map: BTreeMap<(AssetId, AssetId), Decimal> = BTreeMap::new();
		let new_price = Decimal::from(200);
		new_price_map.insert((base_asset, quote_asset), new_price);
		let message = EgressMessages::PriceOracle(new_price_map);
		assert_ok!(OCEX::process_egress_msg(&vec![message]));
	})
}

pub fn update_lmp_score() {
	let total_score = Decimal::from(1000);
	let total_fee_paid = Decimal::from(1000);
	let trading_pair_metrics: TradingPairMetrics = (total_score, total_fee_paid);
	let trader = AccountId32::new([1; 32]);
	let trader_score = Decimal::from(100);
	let trader_fee_paid = Decimal::from(100);
	let mut trader_metrics: TraderMetricsMap<AccountId32> = BTreeMap::new();
	trader_metrics.insert(trader.clone(), (trader_score, trader_fee_paid));
	let mut trading_pair_metrics_map: TradingPairMetricsMap<AccountId32> = BTreeMap::new();
	trading_pair_metrics_map.insert(
		TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) },
		(trader_metrics, trading_pair_metrics),
	);
	assert_ok!(OCEX::update_lmp_scores(&trading_pair_metrics_map));
}

pub fn add_lmp_config() {
	let total_liquidity_mining_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
	let total_trading_rewards: Option<u128> = Some(1000 * UNIT_BALANCE);
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	let trading_pair = TradingPair { base: base_asset, quote: quote_asset };
	// Register trading pair
	crete_base_and_quote_asset();
	register_trading_pair();
	let mut market_weightage = BTreeMap::new();
	market_weightage.insert(trading_pair.clone(), UNIT_BALANCE);
	let market_weightage: Option<BTreeMap<TradingPair, u128>> = Some(market_weightage);
	let mut min_fees_paid = BTreeMap::new();
	min_fees_paid.insert(trading_pair.clone(), UNIT_BALANCE);
	let min_fees_paid: Option<BTreeMap<TradingPair, u128>> = Some(min_fees_paid);
	let mut min_maker_volume = BTreeMap::new();
	min_maker_volume.insert(trading_pair, UNIT_BALANCE);
	let min_maker_volume: Option<BTreeMap<TradingPair, u128>> = Some(min_maker_volume);
	let max_accounts_rewarded: Option<u16> = Some(10);
	let claim_safety_period: Option<u32> = Some(0);
	assert_ok!(OCEX::set_lmp_epoch_config(
		RuntimeOrigin::root(),
		total_liquidity_mining_rewards,
		total_trading_rewards,
		market_weightage,
		min_fees_paid,
		min_maker_volume,
		max_accounts_rewarded,
		claim_safety_period
	));
	OCEX::start_new_epoch();
	OCEX::start_new_epoch();
}

use frame_support::traits::fungible::Mutate;
use polkadex_primitives::fees::FeeConfig;

fn crete_base_and_quote_asset() {
	let quote_asset = AssetId::Asset(1);
	Balances::mint_into(&AccountId32::new([1; 32]), UNIT_BALANCE).unwrap();
	assert_ok!(Assets::create(
		RuntimeOrigin::signed(AccountId32::new([1; 32])),
		parity_scale_codec::Compact(quote_asset.asset_id().unwrap()),
		AccountId32::new([1; 32]),
		One::one()
	));
}

fn register_trading_pair() {
	let base_asset = AssetId::Polkadex;
	let quote_asset = AssetId::Asset(1);
	assert_ok!(OCEX::set_exchange_state(RuntimeOrigin::root(), true));
	assert_ok!(OCEX::register_trading_pair(
		RuntimeOrigin::root(),
		base_asset,
		quote_asset,
		1_0000_0000_u128.into(),
		1_000_000_000_000_000_u128.into(),
		1_000_000_u128.into(),
		1_000_000_000_000_000_u128.into(),
		1_000_000_u128.into(),
		1_0000_0000_u128.into(),
	));
}

fn allowlist_token(token: AssetId) {
	let mut allowlisted_token = <AllowlistedToken<Test>>::get();
	allowlisted_token.try_insert(token).unwrap();
	<AllowlistedToken<Test>>::put(allowlisted_token);
}

fn mint_into_account(account_id: AccountId32) {
	let _result = Balances::deposit_creating(&account_id, 10000000000000000000000);
}

fn mint_into_account_large(account_id: AccountId32) {
	let _result =
		Balances::deposit_creating(&account_id, 1_000_000_000_000_000_000_000_000_000_000);
}

#[allow(dead_code)]
fn create_asset_and_credit(asset_id: u128, account_id: AccountId32) {
	assert_ok!(Assets::create(
		RuntimeOrigin::signed(account_id.clone().into()),
		asset_id.into(),
		account_id.clone().into(),
		100_u128
	));
}

fn create_account_id() -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = MemoryKeystore::new();
	let account_id: AccountId32 = <(dyn Keystore + 'static)>::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id;
}

fn create_proxy_account(path: &str) -> AccountId32 {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = MemoryKeystore::new();
	let account_id: AccountId32 = <(dyn Keystore + 'static)>::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/{}", PHRASE, path)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id;
}

fn create_trade_between_alice_and_bob(price: Decimal, qty: Decimal) -> Trade {
	let order1 = create_order_by_alice(price, qty, 3.into(), OrderStatus::OPEN);
	let order2 = create_order_by_bob(price, qty, 3.into(), OrderStatus::OPEN);
	return Trade { maker: order1, taker: order2, price, amount: qty, time: 2 };
}

fn create_order_by_alice(
	price: Decimal,
	qty: Decimal,
	filled: Decimal,
	status: OrderStatus,
) -> Order {
	let account = get_alice_key_pair().public();
	let account_id = AccountId32::new(account.0);
	let fee_config =
		FeeConfig { maker_fraction: Default::default(), taker_fraction: Default::default() };
	let account_info = AccountInfo {
		main_account: account_id.clone(),
		proxies: BoundedVec::new(),
		balances: Default::default(),
		fee_config,
	};
	<Accounts<Test>>::insert(account_id, account_info);
	let mut order = Order {
		stid: 0,
		client_order_id: H256([1u8; 32]),
		avg_filled_price: 0.into(),
		fee: 0.into(),
		filled_quantity: filled.into(),
		status,
		id: H256::random(),
		user: AccountId::new(account.into()),
		main_account: AccountId::new(account.into()),
		pair: get_trading_pair(),
		side: OrderSide::Bid,
		order_type: OrderType::LIMIT,
		qty,
		price,
		quote_order_qty: 0.into(),
		timestamp: 1,
		overall_unreserved_volume: 0.into(),
		signature: get_random_signature(),
	};
	let payload: OrderPayload = order.clone().into();
	order.signature = get_alice_key_pair().sign(&payload.encode()).into();
	return order;
}

fn create_order_by_bob(
	price: Decimal,
	qty: Decimal,
	filled: Decimal,
	status: OrderStatus,
) -> Order {
	let account = get_bob_key_pair().public();
	let account_id = AccountId32::new(account.0);
	let fee_config =
		FeeConfig { maker_fraction: Default::default(), taker_fraction: Default::default() };
	let account_info = AccountInfo {
		main_account: account_id.clone(),
		proxies: BoundedVec::new(),
		balances: Default::default(),
		fee_config,
	};
	<Accounts<Test>>::insert(account_id, account_info);
	let mut order = Order {
		stid: 0,
		client_order_id: H256([1u8; 32]),
		avg_filled_price: 0.into(),
		fee: 0.into(),
		filled_quantity: filled.into(),
		status,
		id: H256::random(),
		user: AccountId::new(account.into()),
		main_account: AccountId::new(account.into()),
		pair: get_trading_pair(),
		side: OrderSide::Ask,
		order_type: OrderType::LIMIT,
		qty,
		price,
		quote_order_qty: 0.into(),
		timestamp: 1,
		overall_unreserved_volume: 0.into(),
		signature: get_random_signature(),
	};
	let payload: OrderPayload = order.clone().into();
	order.signature = get_bob_key_pair().sign(&payload.encode()).into();
	return order;
}

pub fn get_alice_key_pair() -> sp_core::sr25519::Pair {
	return sp_core::sr25519::Pair::from_string("//Alice", None).unwrap();
}

pub fn get_bob_key_pair() -> sp_core::sr25519::Pair {
	return sp_core::sr25519::Pair::from_string("//Bob", None).unwrap();
}

pub fn get_trading_pair_config() -> TradingPairConfig {
	TradingPairConfig {
		base_asset: get_trading_pair().base,
		quote_asset: get_trading_pair().quote,
		min_price: Decimal::from_str("0.0001").unwrap(),
		max_price: Decimal::from_str("1000").unwrap(),
		price_tick_size: Decimal::from_str("0.000001").unwrap(),
		min_qty: Decimal::from_str("0.001").unwrap(),
		max_qty: Decimal::from_str("1000").unwrap(),
		qty_step_size: Decimal::from_str("0.001").unwrap(),
		operational_status: true,
		base_asset_precision: 8,
		quote_asset_precision: 8,
	}
}

pub fn get_trading_pair() -> TradingPair {
	TradingPair { base: AssetId::Polkadex, quote: AssetId::Asset(1) }
}

pub fn get_random_signature() -> Signature {
	Signature::Ecdsa(Default::default())
}

fn create_max_fees<T: Config>() -> Fees {
	let fees: Fees = Fees { asset: AssetId::Polkadex, amount: Decimal::MAX };
	return fees;
}

pub mod fixture_old_user_action {
	use orderbook_primitives::types::{Trade, WithdrawalRequest};
	use parity_scale_codec::{Codec, Decode, Encode};
	use polkadex_primitives::AccountId;
	use scale_info::TypeInfo;

	#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq)]
	pub enum UserActions<AccountId: Codec + Clone + TypeInfo> {
		/// Trade operation requested.
		Trade(Vec<Trade>),
		/// Withdraw operation requested.
		Withdraw(WithdrawalRequest<AccountId>),
		/// Block import requested.
		BlockImport(u32),
		/// Reset Flag
		Reset,
	}

	pub fn get_old_user_action_fixture() -> Vec<u8> {
		let block_import: UserActions<AccountId> = UserActions::BlockImport(24);
		block_import.encode()
	}
}
