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
use polkadex_primitives::{
	ocex::TradingPairConfig, withdrawal::Withdrawal, AccountId, AssetId, BlockNumber,
};
pub use primitive_types::H128;
use rust_decimal::Decimal;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_core::ByteArray;
use sp_core::H256;
use sp_runtime::traits::IdentifyAccount;
use sp_std::vec::Vec;

pub mod constants;
pub mod types;

/// Defines fees asset to amount map DTO.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, PartialEq)]
pub struct Fees {
	/// Asset identifier.
	pub asset: AssetId,
	/// Amount.
	pub amount: Decimal,
}

/// Defines the structure of snapshot DTO.
#[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq)]
pub struct SnapshotSummary<AccountId: Clone + Codec> {
	/// Snapshot identifier.
	pub snapshot_id: u64,
	/// Worker nonce.
	pub worker_nonce: u64,
	/// State change identifier.
	pub state_change_id: u64,
	/// Latest processed block number.
	pub last_processed_blk: BlockNumber,
	/// Collections of withdrawals.
	pub withdrawals: Vec<Withdrawal<AccountId>>,
	/// SGX report
	pub report: Vec<u8>,
	/// State Hash
	pub state_hash: H256,
}

impl<AccountId: Clone + Codec> Default for SnapshotSummary<AccountId> {
	fn default() -> Self {
		Self {
			snapshot_id: 0,
			worker_nonce: 0,
			state_change_id: 0,
			last_processed_blk: 0,
			withdrawals: Vec::default(),
			report: Vec::default(),
			state_hash: Default::default(),
		}
	}
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
