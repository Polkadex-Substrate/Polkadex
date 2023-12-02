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

//! Helper functions for updating the balance

use crate::storage::OffchainState;
use log::{error, info};
use orderbook_primitives::types::Trade;
use parity_scale_codec::{alloc::string::ToString, Decode, Encode};
use polkadex_primitives::{ocex::TradingPairConfig, AccountId, AssetId};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use sp_core::crypto::ByteArray;
use sp_std::collections::btree_map::BTreeMap;
use polkadex_primitives::fees::FeeConfig;

/// Updates provided trie db with a new balance entry if it is does not contain item for specific
/// account or asset yet, or increments existing item balance.
///
/// # Parameters
///
/// * `state`: Trie db to update.
/// * `account`: Main Account to look for in the db for update.
/// * `asset`:  Asset to look for
/// * `balance`: Amount on which balance should be added.
pub fn add_balance(
	state: &mut OffchainState,
	account: &AccountId,
	asset: AssetId,
	balance: Decimal,
) -> Result<(), &'static str> {
	log::info!(target:"ocex", "adding {:?} asset {:?} from account {:?}", balance.to_f64().unwrap(), asset.to_string(), account);
	let mut balances: BTreeMap<AssetId, Decimal> = match state.get(&account.to_raw_vec())? {
		None => BTreeMap::new(),
		Some(encoded) => BTreeMap::decode(&mut &encoded[..])
			.map_err(|_| "Unable to decode balances for account")?,
	};

	balances
		.entry(asset)
		.and_modify(|total| *total = total.saturating_add(balance))
		.or_insert(balance);

	state.insert(account.to_raw_vec(), balances.encode());
	Ok(())
}

/// Updates provided trie db with reducing balance of account asset if it exists in the db.
///
/// If account asset balance does not exists in the db `AccountBalanceNotFound` error will be
/// returned.
///
/// # Parameters
///
/// * `state`: Trie db to update.
/// * `account`: Main Account to look for in the db for update.
/// * `asset`:  Asset to look for
/// * `balance`: Amount on which balance should be reduced.
pub fn sub_balance(
	state: &mut OffchainState,
	account: &AccountId,
	asset: AssetId,
	balance: Decimal,
) -> Result<(), &'static str> {
	log::info!(target:"ocex", "subtracting {:?} asset {:?} from account {:?}", balance.to_f64().unwrap(), asset.to_string(), account);
	let mut balances: BTreeMap<AssetId, Decimal> = match state.get(&account.to_raw_vec())? {
		None => return Err("Account not found in trie"),
		Some(encoded) => BTreeMap::decode(&mut &encoded[..])
			.map_err(|_| "Unable to decode balances for account")?,
	};

	let account_balance = balances.get_mut(&asset).ok_or("NotEnoughBalance")?;

	if *account_balance < balance {
		log::error!(target:"ocex","Asset found but balance low for asset: {:?}, of account: {:?}",asset, account);
		return Err("NotEnoughBalance")
	}
	*account_balance = account_balance.saturating_sub(balance);

	state.insert(account.to_raw_vec(), balances.encode());

	Ok(())
}

/// Processes a trade between a maker and a taker, updating their order states and balances
/// accordingly.
///
/// # Parameters
///
/// * `state`: A mutable reference to the Offchain State.
/// * `trade`: A `Trade` object representing the trade to process.
/// * `config`: Trading pair configuration DTO.
///
/// # Returns
///
/// A `Result<(), Error>` indicating whether the trade was successfully processed or not.
pub fn process_trade(
	state: &mut OffchainState,
	trade: &Trade,
	config: TradingPairConfig,
	maker_fees: FeeConfig,
	taker_fees: FeeConfig,
) -> Result<(), &'static str> {
	info!(target: "orderbook", "📒 Processing trade: {:?}", trade);
	if !trade.verify(config) {
		error!(target: "orderbook", "📒 Trade verification failed");
		return Err("InvalidTrade")
	}
	// TODO: Handle Fees here, and update the total fees paid, maker volume for LMP calculations
	// Update balances
	let maker_fees = {
		let (maker_asset, mut maker_credit) = trade.credit(true);
		let maker_fees = maker_credit.saturating_mul(maker_fees.maker_fraction);
		maker_credit = maker_credit.saturating_sub(maker_fees);
		add_balance(state, &maker_asset.main, maker_asset.asset, maker_credit)?;

		let (maker_asset, maker_debit) = trade.debit(true);
		sub_balance(state, &maker_asset.main, maker_asset.asset, maker_debit)?;
		maker_fees
	};
	let taker_fees = {
		let (taker_asset, mut taker_credit) = trade.credit(false);
		let taker_fees = taker_credit.saturating_mul(taker_fees.taker_fraction);
		taker_credit = taker_credit.saturating_sub(taker_fees);
		add_balance(state, &taker_asset.main, taker_asset.asset, taker_credit)?;

		let (taker_asset, taker_debit) = trade.debit(false);
		sub_balance(state, &taker_asset.main, taker_asset.asset, taker_debit)?;
		taker_fees
	};

	// TODO: Store trade.price * trade.volume as maker volume for this epoch
	// TODO: Store maker_fees and taker_fees for the corresponding main account for this epoch
	// TODO: Use this for LMP calculations.
	Ok(())
}


