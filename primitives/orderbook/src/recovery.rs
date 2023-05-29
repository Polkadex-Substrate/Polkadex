use std::collections::BTreeMap;

use parity_scale_codec::{Decode, Encode};
use rust_decimal::Decimal;
use serde_with::{json::JsonString, serde_as};

use polkadex_primitives::{AccountId, BlockNumber};

use crate::types::AccountAsset;

/// A struct representing the recovery state of an Order Book.
#[serde_as]
#[derive(Clone, Debug, Encode, Decode, Default, serde::Serialize, serde::Deserialize)]
pub struct ObRecoveryState {
	/// The snapshot ID of the order book recovery state.
	pub snapshot_id: u64,
	/// A `BTreeMap` that maps main account to a vector of proxy account.
	#[serde_as(as = "JsonString<Vec<(JsonString, _)>>")]
	pub account_ids: BTreeMap<AccountId, Vec<AccountId>>,
	/// A `BTreeMap` that maps `AccountAsset`s to `Decimal` balances.
	#[serde_as(as = "JsonString<Vec<(JsonString, _)>>")]
	pub balances: BTreeMap<AccountAsset, Decimal>,
	/// The last block number that was processed by validator.
	pub last_processed_block_number: BlockNumber,
	/// State change id
	pub state_change_id: u64,
	/// worker nonce
	pub worker_nonce: u64,
}
