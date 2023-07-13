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

#![feature(int_roundings)]
#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Codec, Decode, Encode};
use polkadex_primitives::{withdrawal::Withdrawal, AssetId, BlockNumber};
pub use primitive_types::H128;
use rust_decimal::Decimal;
use scale_info::TypeInfo;
use serde::{Serialize, Deserialize};
use sp_core::H256;
use sp_std::vec::Vec;

pub mod constants;
pub mod types;

#[cfg(feature = "std")]
pub mod recovery;

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

/// Defines the structure of snapshot DTO.
#[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq, Serialize, Deserialize)]
pub struct SnapshotSummary<AccountId: Clone + Codec> {
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
}

impl<AccountId: Clone + Codec> SnapshotSummary<AccountId> {
	/// Collects and returns the collection of fees fro for all withdrawals.
	pub fn get_fees(&self) -> Vec<Fees> {
		let mut fees = Vec::new();
		for withdrawal in &self.withdrawals {
			fees.push(Fees { asset: withdrawal.asset, amount: withdrawal.fees });
		}
		fees
	}
}
