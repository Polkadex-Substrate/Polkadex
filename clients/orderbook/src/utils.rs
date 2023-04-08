use crate::error::Error;
use orderbook_primitives::types::AccountAsset;
use parity_scale_codec::{Decode, Encode};
use reference_trie::ExtensionLayout;
use rust_decimal::Decimal;
use trie_db::{TrieDBMut, TrieMut};

pub fn add_balance(
	trie: &mut TrieDBMut<ExtensionLayout>,
	account_asset: AccountAsset,
	balance: Decimal,
) -> Result<(), Error> {
	match trie.get(&account_asset.encode())? {
		None => {
			// A new account can be created on credit
			trie.insert(&account_asset.encode(), &balance.encode())?;
		},
		Some(data) => {
			let mut account_balance = Decimal::decode(&mut &data[..])?;
			account_balance = account_balance.saturating_add(balance);
			trie.insert(&account_asset.encode(), &account_balance.encode())?;
		},
	}
	Ok(())
}

pub fn sub_balance(
	trie: &mut TrieDBMut<ExtensionLayout>,
	account_asset: AccountAsset,
	balance: Decimal,
) -> Result<(), Error> {
	// We have to throw error if account not found because we expected
	// the account to have balance and exist in the state.
	let data = trie
		.get(&account_asset.encode())?
		.ok_or(Error::AccountBalanceNotFound(account_asset.clone()))?;
	let mut account_balance = Decimal::decode(&mut &data[..])?;
	if account_balance < balance {
		return Err(Error::InsufficientBalance)
	}
	account_balance = account_balance.saturating_sub(balance);
	trie.insert(&account_asset.encode(), &account_balance.encode())?;
	Ok(())
}
