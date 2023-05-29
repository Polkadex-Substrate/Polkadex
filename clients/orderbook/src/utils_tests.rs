use memory_db::{HashKey, MemoryDB};
use parity_scale_codec::{Decode, Encode};
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

use orderbook_primitives::types::AccountAsset;
use polkadex_primitives::{AccountId, AssetId};

use crate::{
	error::Error,
	utils::{add_balance, sub_balance},
};

#[test]
fn test_add_balance_creates_new_free_balance() {
	let mut db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = MemoryDB::default();
	let mut working_state_root = [0u8; 32];
	let mut db_client = get_trie_db_client(&mut db, &mut working_state_root);
	let new_account = AccountId::new([1; 32]);
	let account_asset = AccountAsset { main: new_account, asset: AssetId::Polkadex };
	assert_eq!(
		add_balance(
			&mut db_client,
			account_asset.clone(),
			Decimal::from_u128(1_000_000_000_000u128).unwrap(),
		),
		Ok(())
	);
	let actual_balance = get_balance(&mut db_client, account_asset);
	let expected_balance = Decimal::from_u128(1_000_000_000_000u128).unwrap();
	assert_eq!(actual_balance, expected_balance);
}

#[test]
fn test_add_balance_updates_existing_balance() {
	let mut db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = MemoryDB::default();
	let mut working_state_root = [0u8; 32];
	let mut db_client = get_trie_db_client(&mut db, &mut working_state_root);
	let new_account = AccountId::new([1; 32]);
	let account_asset = AccountAsset { main: new_account, asset: AssetId::Polkadex };
	add_balance(
		&mut db_client,
		account_asset.clone(),
		Decimal::from_u128(1_000_000_000_000u128).unwrap(),
	)
	.unwrap();
	let add_amount = 1_000_000_000_000u128;
	assert_eq!(
		add_balance(&mut db_client, account_asset.clone(), Decimal::from_u128(add_amount).unwrap()),
		Ok(())
	);
	let actual_balance = get_balance(&mut db_client, account_asset);
	let expected_balance = Decimal::from_u128(2_000_000_000_000u128).unwrap();
	assert_eq!(actual_balance, expected_balance);
}

#[test]
fn test_sub_balance_updates_balance() {
	let mut db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = MemoryDB::default();
	let mut working_state_root = [0u8; 32];
	let mut db_client = get_trie_db_client(&mut db, &mut working_state_root);
	let new_account = AccountId::new([1; 32]);
	let account_asset = AccountAsset { main: new_account, asset: AssetId::Polkadex };
	add_balance(
		&mut db_client,
		account_asset.clone(),
		Decimal::from_u128(2_000_000_000_000u128).unwrap(),
	)
	.unwrap();
	let reduce_balance = 1_000_000_000_000u128;
	assert_eq!(
		sub_balance(
			&mut db_client,
			account_asset.clone(),
			Decimal::from_u128(reduce_balance).unwrap(),
		),
		Ok(())
	);
	let actual_balance = get_balance(&mut db_client, account_asset);
	let expected_balance = Decimal::from_u128(1_000_000_000_000u128).unwrap();
	assert_eq!(actual_balance, expected_balance);
}

#[test]
fn test_sub_balance_returns_account_not_found() {
	let mut db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = MemoryDB::default();
	let mut working_state_root = [0u8; 32];
	let mut db_client = get_trie_db_client(&mut db, &mut working_state_root);
	let new_account = AccountId::new([1; 32]);
	let account_asset = AccountAsset { main: new_account, asset: AssetId::Polkadex };
	let reduce_balance = 1_000_000_000_000u128;
	assert_eq!(
		sub_balance(
			&mut db_client,
			account_asset.clone(),
			Decimal::from_u128(reduce_balance).unwrap(),
		),
		Err(Error::AccountBalanceNotFound(account_asset))
	);
}

#[test]
fn test_sub_balance_returns_insufficient_balance() {
	let mut db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = MemoryDB::default();
	let mut working_state_root = [0u8; 32];
	let mut db_client = get_trie_db_client(&mut db, &mut working_state_root);
	let new_account = AccountId::new([1; 32]);
	let account_asset = AccountAsset { main: new_account, asset: AssetId::Polkadex };
	add_balance(
		&mut db_client,
		account_asset.clone(),
		Decimal::from_u128(2_000_000_000_000u128).unwrap(),
	)
	.unwrap();
	let reduce_balance = 3_000_000_000_000u128;
	assert_eq!(
		sub_balance(
			&mut db_client,
			account_asset.clone(),
			Decimal::from_u128(reduce_balance).unwrap(),
		),
		Err(Error::InsufficientBalance)
	);
}

fn get_trie_db_client<'a>(
	memory_db: &'a mut MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>,
	working_state_root: &'a mut [u8; 32],
) -> TrieDBMut<'a, ExtensionLayout> {
	let trie = TrieDBMutBuilder::new(memory_db, working_state_root).build();
	trie
}

fn get_balance(client: &TrieDBMut<ExtensionLayout>, account_asset: AccountAsset) -> Decimal {
	let db_value = client.get(&account_asset.encode()).unwrap().unwrap();
	let account_balance = Decimal::decode(&mut &db_value[..]).unwrap();
	account_balance
}
