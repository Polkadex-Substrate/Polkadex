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

//! # Orderbook Primitives.
//!
//! This crate contains common types and operations definition required for the `Orderbook` related
//! components.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use crate::recovery::ObCheckpoint;
use crate::types::{AccountAsset, TradingPair};
use frame_support::dispatch::DispatchResult;
use parity_scale_codec::{Codec, Decode, Encode};
use polkadex_primitives::{
	ingress::EgressMessages, withdrawal::Withdrawal, AssetId, BlockNumber, UNIT_BALANCE,
};
pub use primitive_types::H128;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use scale_info::TypeInfo;
use serde::{Deserialize, Serialize};
use sp_core::H256;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

pub mod constants;
pub mod types;

pub mod lmp;
#[cfg(feature = "std")]
pub mod recovery;
pub mod traits;

/// Authority set id starts with zero at genesis.
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// A typedef for validator set id.
pub type ValidatorSetId = u64;

/// A set of Orderbook authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct ValidatorSet<AuthorityId> {
	/// Validator Set id.
	pub set_id: ValidatorSetId,
	/// Public keys of the validator set elements.
	pub validators: Vec<AuthorityId>,
}

impl<AuthorityId> Default for ValidatorSet<AuthorityId> {
	fn default() -> Self {
		ValidatorSet { set_id: GENESIS_AUTHORITY_SET_ID, validators: Vec::new() }
	}
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return a validator set with the given validators and set id.
	pub fn new<I>(validators: I, set_id: ValidatorSetId) -> Self
	where
		I: IntoIterator<Item = AuthorityId>,
	{
		let validators: Vec<AuthorityId> = validators.into_iter().collect();
		Self { set_id, validators }
	}

	/// Return a reference to the vec of validators.
	pub fn validators(&self) -> &[AuthorityId] {
		&self.validators
	}

	/// Return the number of validators in the set.
	pub fn len(&self) -> usize {
		self.validators.len()
	}

	/// Return true if set is empty
	pub fn is_empty(&self) -> bool {
		self.validators.is_empty()
	}
}

/// The index of an authority.
pub type AuthorityIndex = u32;

/// Defines fees asset to amount map DTO.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, PartialEq)]
pub struct Fees {
	/// Asset identifier.
	pub asset: AssetId,
	/// Amount.
	pub amount: Decimal,
}

impl Fees {
	pub fn amount(&self) -> u128 {
		self.amount
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.unwrap_or_default() // this shouldn't fail.
	}
}

pub type TotalScore = Decimal;
pub type TotalFeePaid = Decimal;
pub type Score = Decimal;
pub type FeePaid = Decimal;
pub type TraderMetrics = (Score, FeePaid);
pub type TraderMetricsMap<AccountId> = BTreeMap<AccountId, TraderMetrics>;
pub type TradingPairMetrics = (TotalScore, TotalFeePaid);
pub type TradingPairMetricsMap<AccountId> =
	BTreeMap<TradingPair, (TraderMetricsMap<AccountId>, TradingPairMetrics)>;

/// Defines the structure of snapshot DTO.
#[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub struct SnapshotSummary<AccountId: Clone + Codec + Ord> {
	/// Validator set identifier.
	pub validator_set_id: u64,
	/// Snapshot identifier.
	pub snapshot_id: u64,
	/// Working state root.
	pub state_hash: H256,
	/// State change identifier.
	pub state_change_id: u64,
	/// Latest processed block number.
	pub last_processed_blk: BlockNumber,
	/// Collections of withdrawals.
	pub withdrawals: Vec<Withdrawal<AccountId>>,
	/// List of Egress messages
	pub egress_messages: Vec<EgressMessages<AccountId>>,
	/// Trader Metrics
	pub trader_metrics: Option<TradingPairMetricsMap<AccountId>>,
}

impl<AccountId: Clone + Codec + Ord> SnapshotSummary<AccountId> {
	/// Collects and returns the collection of fees fro for all withdrawals.
	pub fn get_fees(&self) -> Vec<Fees> {
		let mut fees = Vec::new();
		for withdrawal in &self.withdrawals {
			fees.push(Fees { asset: withdrawal.asset, amount: withdrawal.fees });
		}
		fees
	}
}

#[derive(Clone, Debug, Encode, Decode, Default, TypeInfo)]
pub struct ObCheckpointRaw {
	/// The snapshot ID of the order book recovery state.
	pub snapshot_id: u64,
	/// A `BTreeMap` that maps `AccountAsset`s to `Decimal` balances.
	pub balances: BTreeMap<AccountAsset, Decimal>,
	/// The last block number that was processed by validator.
	pub last_processed_block_number: BlockNumber,
	/// State change id
	pub state_change_id: u64,
}

impl ObCheckpointRaw {
	/// Create a new `ObCheckpointRaw` instance.
	/// # Parameters
	/// * `snapshot_id`: The snapshot ID of the order book recovery state.
	/// * `balances`: A `BTreeMap` that maps `AccountAsset`s to `Decimal` balances.
	/// * `last_processed_block_number`: The last block number that was processed by validator.
	/// * `state_change_id`: State change id
	/// # Returns
	/// * `ObCheckpointRaw`: A new `ObCheckpointRaw` instance.
	pub fn new(
		snapshot_id: u64,
		balances: BTreeMap<AccountAsset, Decimal>,
		last_processed_block_number: BlockNumber,
		state_change_id: u64,
	) -> Self {
		Self { snapshot_id, balances, last_processed_block_number, state_change_id }
	}

	/// Convert `ObCheckpointRaw` to `ObCheckpoint`.
	/// # Returns
	/// * `ObCheckpoint`: A new `ObCheckpoint` instance.
	#[cfg(feature = "std")]
	pub fn to_checkpoint(self) -> ObCheckpoint {
		ObCheckpoint {
			snapshot_id: self.snapshot_id,
			balances: self.balances,
			last_processed_block_number: self.last_processed_block_number,
			state_change_id: self.state_change_id,
		}
	}
}

pub trait LiquidityMining<AccountId, Balance> {
	/// Registers the pool_id as main account, trading account.
	fn register_pool(pool_id: AccountId, trading_account: AccountId) -> DispatchResult;

	/// Returns the Current Average price
	fn average_price(market: TradingPair) -> Option<Decimal>;
	/// Returns if its a registered market in OCEX pallet
	fn is_registered_market(market: &TradingPair) -> bool;

	/// Deposits the given amounts to Orderbook and Adds an ingress message requesting engine to
	/// calculate the exact shares and return it as an egress message
	fn add_liquidity(
		market: TradingPair,
		pool: AccountId,
		lp: AccountId,
		total_shares_issued: Decimal,
		base_amount: Decimal,
		quote_amount: Decimal,
	) -> DispatchResult;

	/// Adds an ingress message to initiate withdrawal request and queue it for execution at the end
	/// of cycle.
	fn remove_liquidity(
		market: TradingPair,
		pool: AccountId,
		lp: AccountId,
		given: Balance,
		total: Balance,
	);

	/// Adds an ingress message to force close all open orders from this main account and initiate
	/// complete withdrawal
	fn force_close_pool(market: TradingPair, main: AccountId);

	/// Claim rewards for this main account. Return False if reward is already claimed, else True.
	fn claim_rewards(
		main: AccountId,
		epoch: u16,
		market: TradingPair,
	) -> Result<Balance, sp_runtime::DispatchError>;
}
