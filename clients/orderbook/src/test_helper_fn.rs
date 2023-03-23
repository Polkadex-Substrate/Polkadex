use crate::{
	error::Error,
	worker::{add_proxy, deposit, process_trade, register_main, remove_proxy},
};
use log::trace;
use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::types::{
	AccountAsset, AccountInfo, Order, OrderSide, OrderType, Trade, TradingPair,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ingress::IngressMessages, AccountId, AssetId};
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use sp_core::{blake2_128, offchain::OffchainStorage, Bytes, Pair, H160, H256};
use sp_keyring::AccountKeyring;
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

pub fn get_alice_main_and_proxy_account() -> (AccountId, AccountId) {
	let main_account = AccountId::from(AccountKeyring::Alice.pair().public());
	let proxy_account = AccountId::from([1_u8; 32]);
	(main_account, proxy_account)
}

pub fn get_bob_main_and_proxy_account() -> (AccountId, AccountId) {
	let main_account = AccountId::from(AccountKeyring::Bob.pair().public());
	let proxy_account = AccountId::from([5_u8; 32]);
	(main_account, proxy_account)
}

#[test]
pub fn register_main_account() {
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

#[test]
pub fn re_register_main_account() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	let result = register_main(&mut trie, alice_main.clone(), alice_proxy.clone());
	assert_eq!(result, Err(Error::MainAlreadyRegistered).into());
}

#[test]
pub fn add_proxy_account() {
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

#[test]
pub fn add_duplicate_proxy_account() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	assert!(register_main(&mut trie, alice_main.clone(), alice_proxy.clone()).is_ok());
	let result = add_proxy(&mut trie, alice_main.clone(), alice_proxy.clone());
	assert_eq!(result, Err(Error::ProxyAlreadyRegistered).into());
}

#[test]
pub fn add_proxy_account_when_main_account_not_register() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	let result = add_proxy(&mut trie, alice_main.clone(), alice_proxy.clone());
	assert_eq!(result, Err(Error::MainAccountNotFound).into());
}

#[test]
pub fn remove_proxy_account() {
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

#[test]
pub fn remove_proxy_account_when_main_account_not_register() {
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

#[test]
pub fn remove_unregister_proxy_account() {
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

#[test]
pub fn deposit_when_main_account_not_register() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, _alice_proxy) = get_alice_main_and_proxy_account();
	let result = deposit(&mut trie, alice_main.clone(), AssetId::asset(1), Decimal::new(10, 0));
	assert_eq!(result, Err(Error::MainAccountNotFound).into());
}

#[test]
pub fn deposit_asset() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	let asset_id = AssetId::asset(1);
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

#[test]
pub fn process_a_trade() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	let mut trie: TrieDBMut<ExtensionLayout> =
		TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();
	let (alice_main, alice_proxy) = get_alice_main_and_proxy_account();
	let (bob_main, bob_proxy) = get_bob_main_and_proxy_account();

	let asset_id_1 = AssetId::asset(1);
	let asset_id_2 = AssetId::asset(1);

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

	let mut bob_bid_limit_order =
		Order::random_order_for_testing(trading_pair, OrderSide::Bid, OrderType::LIMIT);
	bob_bid_limit_order.price = Decimal::from(1_u32);
	bob_bid_limit_order.qty = Decimal::from(2_u32);
	bob_bid_limit_order.user = bob_proxy.clone();
	bob_bid_limit_order.main_account = bob_main.clone();

	let trade =
		Trade::new(bob_bid_limit_order, alice_ask_limit_order, Decimal::from(1), Decimal::from(2));

	assert!(process_trade(&mut trie, trade.clone()).is_ok());
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
