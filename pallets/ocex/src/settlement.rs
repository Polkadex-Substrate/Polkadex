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

//! Contains common/reusable functionality.

use crate::validator::map_trie_error;
use log::{error, info};
use orderbook_primitives::types::Trade;
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ocex::TradingPairConfig, AccountId, AssetId};
use rust_decimal::Decimal;
use sp_core::crypto::ByteArray;
use sp_runtime::traits::BlakeTwo256;
use sp_std::collections::btree_map::BTreeMap;
use sp_trie::LayoutV1;
use trie_db::{TrieDBMut, TrieMut};

/// Updates provided trie db with a new entrance balance if it is not contains item for specific
/// account asset yet, or increments existed item balance.
///
/// # Parameters
///
/// * `trie`: Trie db to update.
/// * `account_asset`: Account asset to look for in the db for update.
/// * `balance`: Amount on which account asset balance should be incremented.
pub fn add_balance(
	state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	account: &AccountId,
	asset: AssetId,
	balance: Decimal,
) -> Result<(), &'static str> {
	let mut balances: BTreeMap<AssetId, Decimal> =
		match state.get(account.as_slice()).map_err(map_trie_error)? {
			None => BTreeMap::new(),
			Some(encoded) => BTreeMap::decode(&mut &encoded[..])
				.map_err(|_| "Unable to decode balances for account")?,
		};

	balances
		.entry(asset)
		.and_modify(|total| *total = total.saturating_add(balance))
		.or_insert(balance);

	state.insert(account.as_slice(), &balances.encode()).map_err(map_trie_error)?;
	Ok(())
}

/// Updates provided trie db with reducing balance of account asset if it exists in the db.
///
/// If account asset balance is not exists in the db `AccountBalanceNotFound` error will be
/// returned.
///
/// # Parameters
///
/// * `trie`: Trie db to update.
/// * `account_asset`: Account asset to look for in the db for update.
/// * `balance`: Amount on which account asset balance should be reduced.
pub fn sub_balance(
	state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	account: &AccountId,
	asset: AssetId,
	balance: Decimal,
) -> Result<(), &'static str> {
	info!(target:"orderbook","📒 Subtracting balance from account");

	let mut balances: BTreeMap<AssetId, Decimal> =
		match state.get(account.as_slice()).map_err(map_trie_error)? {
			None => return Err("Account not found in trie"),
			Some(encoded) => BTreeMap::decode(&mut &encoded[..])
				.map_err(|_| "Unable to decode balances for account")?,
		};

	let account_balance = balances.get_mut(&asset).ok_or("NotEnoughBalance")?;

	if *account_balance < balance {
		return Err("NotEnoughBalance")
	}
	*account_balance = account_balance.saturating_sub(balance);

	state.insert(account.as_slice(), &balances.encode()).map_err(map_trie_error)?;

	Ok(())
}

/// Processes a trade between a maker and a taker, updating their order states and balances
/// accordingly.
///
/// # Parameters
///
/// * `accounts`: A mutable reference to a Accounts.
/// * `trade`: A `Trade` object representing the trade to process.
/// * `config`: Trading pair configuration DTO.
///
/// # Returns
///
/// A `Result<(), Error>` indicating whether the trade was successfully processed or not.
pub fn process_trade(
	state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	trade: &Trade,
	config: TradingPairConfig,
) -> Result<(), &'static str> {
	info!(target: "orderbook", "📒 Processing trade: {:?}", trade);
	if !trade.verify(config) {
		error!(target: "orderbook", "📒 Trade verification failed");
		return Err("InvalidTrade")
	}

	// Update balances
	{
		let (maker_asset, maker_credit) = trade.credit(true);
		add_balance(state, &maker_asset.main, maker_asset.asset, maker_credit)?;

		let (maker_asset, maker_debit) = trade.debit(true);
		sub_balance(state, &maker_asset.main, maker_asset.asset, maker_debit)?;
	}
	{
		let (taker_asset, taker_credit) = trade.credit(false);
		add_balance(state, &taker_asset.main, taker_asset.asset, taker_credit)?;

		let (taker_asset, taker_debit) = trade.debit(false);
		sub_balance(state, &taker_asset.main, taker_asset.asset, taker_debit)?;
	}
	Ok(())
}
