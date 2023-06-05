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

//! This module contains code that defines test cases related to a trading system of worker module.

use crate::{
	error::Error,
	worker::{add_proxy, deposit, process_trade, register_main, remove_proxy},
};
use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::types::{
	AccountAsset, AccountInfo, Order, OrderPayload, OrderSide, OrderType, Trade, TradingPair,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ocex::TradingPairConfig, AccountId, AssetId, Signature};
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::Decimal;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

/// This function returns a tuple containing Alice's main account and a proxy account.
fn get_alice_main_and_proxy_account() -> (AccountId, AccountId) {
	let main_account = AccountId::from(AccountKeyring::Alice.pair().public());
	let proxy_account = AccountId::from(AccountKeyring::Charlie.pair().public());
	(main_account, proxy_account)
}

/// This function returns a tuple containing Bob's main account and a proxy account.
fn get_bob_main_and_proxy_account() -> (AccountId, AccountId) {
	let main_account = AccountId::from(AccountKeyring::Bob.pair().public());
	let proxy_account = AccountId::from(AccountKeyring::Eve.pair().public());
	(main_account, proxy_account)
}

/// Register main account and assert changes in db.
#[test]
pub fn register_main_will_store_successfully() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	trie.commit();
	let get_db_val = trie.get(&alice_main.encode()).unwrap().unwrap().to_vec().clone();
	let account_info_in_db = AccountInfo::decode(&mut &get_db_val[..]).unwrap();
	assert_eq!(account_info_in_db.proxies, vec![alice_proxy]);
}

/// Add proxy account and assert changes in db.
#[test]
pub fn add_proxy_will_store_it_successfully() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	let alice_new_proxy_account = AccountId::from([2_u8; 32]);
	assert!(add_proxy(&mut trie, alice_main.clone(), alice_new_proxy_account.clone()).is_ok());

	let get_db_val = trie.get(&alice_main.encode()).unwrap().unwrap().to_vec().clone();
	let account_info_in_db = AccountInfo::decode(&mut &get_db_val[..]).unwrap();
	assert_eq!(account_info_in_db.proxies, vec![alice_proxy, alice_new_proxy_account]);
}

/// Remove proxy account and assert changes in db.
#[test]
pub fn remove_proxy_will_remove_it_from_the_storage_successfully() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	assert!(remove_proxy(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());

	let get_db_val = trie.get(&alice_main.encode()).unwrap().unwrap().to_vec().clone();
	let account_info_in_db = AccountInfo::decode(&mut &get_db_val[..]).unwrap();
	assert_eq!(account_info_in_db.proxies, vec![]);
}

/// Try to remove a proxy account when main account not registered.
#[test]
pub fn remove_proxy_with_not_registered_main_account_will_return_main_account_not_found_error() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert_eq!(
		remove_proxy(&mut trie, alice_main.clone(), alice_proxy.clone()),
		Err(Error::MainAccountNotFound).into()
	);
}

/// Try to remove a non registered proxy account and assert expected error.
#[test]
pub fn remove_proxy_with_not_registered_proxy_will_return_proxy_account_not_found_error() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	assert_eq!(
		remove_proxy(&mut trie, alice_main.clone(), AccountId::from([2_u8; 32])),
		Err(Error::ProxyAccountNotFound).into()
	);
}

/// Try to deposit a amount when main account is not register and assert expected error.
#[test]
pub fn deposit_with_not_registered_main_account_will_return_main_account_not_found_error() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, _alice_proxy) = get_alice_main_and_proxy_account();
	let result = deposit(&mut trie, alice_main.clone(), AssetId::Asset(1), Decimal::new(10, 0));
	assert_eq!(result, Err(Error::MainAccountNotFound).into());
}

/// Deposit assets in users main account and assert changes in DB.
#[test]
pub fn deposit_will_store_amount_successfully() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	let asset_id = AssetId::Asset(1);
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	assert!(deposit(&mut trie, alice_main.clone(), asset_id.clone(), Decimal::new(10, 0)).is_ok());

	let account_asset = AccountAsset { main: alice_main.clone(), asset: asset_id };

	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let balance = Decimal::decode(&mut &get_db_val[..]).unwrap();
	assert_eq!(balance, Decimal::new(10, 0));

	// Redeposit
	assert!(deposit(&mut trie, alice_main.clone(), asset_id.clone(), Decimal::new(10, 0)).is_ok());
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let balance = Decimal::decode(&mut &get_db_val[..]).unwrap();
	assert_eq!(balance, Decimal::new(20, 0));
}

/// Process a receive trade and assert balance changes in DB.
#[test]
pub fn process_trade_will_process_successfully() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	let (bob_main, bob_proxy) = get_bob_main_and_proxy_account();

	let asset_id_1 = AssetId::Asset(1);
	let asset_id_2 = AssetId::Asset(1);

	//Register alice & deposit asset 1 & 2
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	assert!(deposit(&mut trie, alice_main.clone(), asset_id_1.clone(), Decimal::new(10, 0)).is_ok());
	assert!(deposit(&mut trie, alice_main.clone(), asset_id_2.clone(), Decimal::new(10, 0)).is_ok());

	//Register bob & deposit asset 1 & 2
	assert!(register_main(&mut trie, bob_main.clone(), bob_proxy.clone()).is_ok());
	assert!(deposit(&mut trie, bob_main.clone(), asset_id_1.clone(), Decimal::new(10, 0)).is_ok());
	assert!(deposit(&mut trie, bob_main.clone(), asset_id_2.clone(), Decimal::new(10, 0)).is_ok());
	trie.commit();

	//getting balances
	let account_asset = AccountAsset { main: alice_main.clone(), asset: asset_id_1.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let alice_balance_asset_1 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	let account_asset = AccountAsset { main: alice_main.clone(), asset: asset_id_2.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let alice_balance_asset_2 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	let account_asset = AccountAsset { main: bob_main.clone(), asset: asset_id_1.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let bob_balance_asset_1 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	let account_asset = AccountAsset { main: bob_main.clone(), asset: asset_id_2.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let bob_balance_asset_2 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	//asserting balances
	assert_eq!(alice_balance_asset_1, Decimal::from(20));
	assert_eq!(alice_balance_asset_2, Decimal::from(20));
	assert_eq!(bob_balance_asset_1, Decimal::from(20));
	assert_eq!(bob_balance_asset_2, Decimal::from(20));

	let trading_pair = TradingPair { base: asset_id_1, quote: asset_id_2 };

	let mut alice_ask_limit_order =
		Order::random_order_for_testing(trading_pair, OrderSide::Ask, OrderType::LIMIT);
	alice_ask_limit_order.price = Decimal::from(1_u32);
	alice_ask_limit_order.qty = Decimal::from(2_u32);
	alice_ask_limit_order.user = alice_proxy.clone();
	alice_ask_limit_order.main_account = alice_main.clone();

	alice_ask_limit_order.signature = Signature::from(
		AccountKeyring::Charlie
			.pair()
			.sign(&OrderPayload::from(alice_ask_limit_order.clone()).encode()[..]),
	);

	let mut bob_bid_limit_order =
		Order::random_order_for_testing(trading_pair, OrderSide::Bid, OrderType::LIMIT);
	bob_bid_limit_order.price = Decimal::from(1_u32);
	bob_bid_limit_order.qty = Decimal::from(2_u32);
	bob_bid_limit_order.user = bob_proxy.clone();
	bob_bid_limit_order.main_account = bob_main.clone();

	bob_bid_limit_order.signature = Signature::from(
		AccountKeyring::Eve
			.pair()
			.sign(&OrderPayload::from(bob_bid_limit_order.clone()).encode()[..]),
	);

	let trade =
		Trade::new(bob_bid_limit_order, alice_ask_limit_order, Decimal::from(1), Decimal::from(2));

	let config = TradingPairConfig::default(trading_pair.base, trading_pair.quote);
	assert!(process_trade(&mut trie, trade.clone(), config).is_ok());
	trie.commit();

	//getting balances
	let account_asset = AccountAsset { main: alice_main.clone(), asset: asset_id_1.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let alice_balance_asset_1 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	let account_asset = AccountAsset { main: alice_main.clone(), asset: asset_id_2.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let alice_balance_asset_2 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	let account_asset = AccountAsset { main: bob_main.clone(), asset: asset_id_1.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let bob_balance_asset_1 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	let account_asset = AccountAsset { main: bob_main.clone(), asset: asset_id_2.clone() };
	let get_db_val = trie.get(&account_asset.encode()).unwrap().unwrap().to_vec().clone();
	let bob_balance_asset_2 = Decimal::decode(&mut &get_db_val[..]).unwrap();

	//asserting balances
	assert_eq!(alice_balance_asset_1, Decimal::from(20));
	assert_eq!(alice_balance_asset_2, Decimal::from(20));
	assert_eq!(bob_balance_asset_1, Decimal::from(20));
	assert_eq!(bob_balance_asset_2, Decimal::from(20));
}
