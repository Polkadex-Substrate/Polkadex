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

use crate::snapshot::AccountsMap;
use log::{error, info};
use orderbook_primitives::types::Trade;
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ocex::TradingPairConfig, AccountId, AssetId, BlockNumber};
use rust_decimal::Decimal;
use sp_runtime::{traits::Zero, Saturating};
use sp_std::collections::btree_map::BTreeMap;

/// Updates provided trie db with a new entrance balance if it is not contains item for specific
/// account asset yet, or increments existed item balance.
///
/// # Parameters
///
/// * `trie`: Trie db to update.
/// * `account_asset`: Account asset to look for in the db for update.
/// * `balance`: Amount on which account asset balance should be incremented.
pub fn add_balance(
	account: &mut BTreeMap<AssetId, Decimal>,
	asset: AssetId,
	balance: Decimal,
) -> Result<(), &'static str> {
	account
		.entry(asset)
		.and_modify(|total| *total = total.saturating_add(balance))
		.or_insert(balance);

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
	account: &mut BTreeMap<AssetId, Decimal>,
	asset: AssetId,
	balance: Decimal,
) -> Result<(), &'static str> {
	info!(target:"orderbook","📒 Subtracting balance from account");

	let account_balance = account.get_mut(&asset).ok_or("NotEnoughBalance")?;

	if *account_balance < balance {
		return Err("NotEnoughBalance")
	}
	*account_balance = account_balance.saturating_sub(balance);

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
	accounts: &mut AccountsMap,
	trade: Trade,
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
		let account_info = accounts
			.balances
			.get_mut(&maker_asset.main.clone().into())
			.ok_or("MainAccountNotFound")?;
		add_balance(account_info, maker_asset.asset, maker_credit)?;

		let (maker_asset, maker_debit) = trade.debit(true);
		sub_balance(account_info, maker_asset.asset, maker_debit)?;
	}
	{
		let (taker_asset, taker_credit) = trade.credit(false);
		let account_info = accounts
			.balances
			.get_mut(&taker_asset.main.clone().into())
			.ok_or("MainAccountNotFound")?;
		add_balance(account_info, taker_asset.asset, taker_credit)?;

		let (taker_asset, taker_debit) = trade.debit(false);
		sub_balance(account_info, taker_asset.asset, taker_debit)?;
	}
	Ok(())
}
