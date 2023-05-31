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

use crate::error::Error;
use orderbook_primitives::types::AccountAsset;
use parity_scale_codec::{Decode, Encode};
use reference_trie::ExtensionLayout;
use rust_decimal::Decimal;
use sp_tracing::info;
use trie_db::{TrieDBMut, TrieMut};

pub fn add_balance(
	trie: &mut TrieDBMut<ExtensionLayout>,
	account_asset: AccountAsset,
	balance: Decimal,
) -> Result<(), Error> {
	match trie.get(&account_asset.encode())? {
		None => {
			info!(target:"orderbook", "📒 Account not found, creating new account");
			// A new account can be created on credit
			trie.insert(&account_asset.encode(), &balance.encode())?;
		},
		Some(data) => {
			info!(target:"orderbook","📒 Account already exists, adding balance to it");
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
	info!(target:"orderbook","📒 Subtracting balance from account");
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
