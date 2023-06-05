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

use crate::crypto::AuthorityId;
use bls_primitives::{Public, Signature};
use parity_scale_codec::{Codec, Decode, Encode};
use polkadex_primitives::{
	ocex::TradingPairConfig,
	utils::{return_set_bits, set_bit_field},
	withdrawal::Withdrawal,
	AccountId, AssetId, BlockNumber,
};
use primitive_types::H128;
use rust_decimal::Decimal;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_core::ByteArray;
use sp_core::H256;
use sp_runtime::traits::IdentifyAccount;
use sp_std::vec::Vec;

pub mod constants;
pub mod types;

#[cfg(feature = "std")]
pub mod recovery;

/// Key type for Orderbook module.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"orbk");

/// Orderbook cryptographic types
///
/// This module basically introduces three crypto types:
/// - `crypto::Pair`
/// - `crypto::Public`
/// - `crypto::Signature`
///
/// Your code should use the above types as concrete types for all crypto related
/// functionality.
///
/// The current underlying crypto scheme used is BLS. This can be changed,
/// without affecting code restricted against the above listed crypto types.
pub mod crypto {
	use sp_application_crypto::app_crypto;

	use bls_primitives as BLS;

	app_crypto!(BLS, crate::KEY_TYPE);

	/// Identity of a Orderbook authority using BLS as its crypto.
	pub type AuthorityId = Public;

	/// Signature for a Orderbook authority using BLS as its crypto.
	pub type AuthoritySignature = Signature;
}

impl IdentifyAccount for AuthorityId {
	type AccountId = Self;
	fn into_account(self) -> Self {
		self
	}
}

#[cfg(feature = "std")]
impl TryFrom<[u8; 96]> for crypto::AuthorityId {
	type Error = ();
	fn try_from(value: [u8; 96]) -> Result<Self, Self::Error> {
		crypto::AuthorityId::from_slice(&value)
	}
}

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
#[derive(Clone, Encode, Decode, Debug, TypeInfo, PartialEq)]
pub struct SnapshotSummary<AccountId: Clone + Codec> {
	/// Validator set identifier.
	pub validator_set_id: u64,
	/// Snapshot identifier.
	pub snapshot_id: u64,
	/// Working state root.
	pub state_root: H256,
	/// Worker nonce.
	pub worker_nonce: u64,
	/// State change identifier.
	pub state_change_id: u64,
	/// Latest processed block number.
	pub last_processed_blk: BlockNumber,
	/// State chunk hashes.
	pub state_chunk_hashes: Vec<H128>,
	/// Bitmap.
	pub bitflags: Vec<u128>,
	/// Collections of withdrawals.
	pub withdrawals: Vec<Withdrawal<AccountId>>,
	/// Aggregated signature.
	pub aggregate_signature: Option<bls_primitives::Signature>,
}

impl<AccountId: Clone + Codec> Default for SnapshotSummary<AccountId> {
	fn default() -> Self {
		Self {
			validator_set_id: 0,
			snapshot_id: 0,
			state_root: Default::default(),
			worker_nonce: 0,
			state_change_id: 0,
			last_processed_blk: 0,
			state_chunk_hashes: Vec::new(),
			bitflags: Vec::new(),
			withdrawals: Vec::new(),
			aggregate_signature: None,
		}
	}
}

impl<AccountId: Clone + Codec> SnapshotSummary<AccountId> {
	/// Adds a new signature to the snapshot summary.
	///
	/// # Parameters
	///
	/// * `signature`: Signature to add.
	pub fn add_signature(&mut self, signature: Signature) -> Result<(), Signature> {
		let aggregate_signature = self.aggregate_signature.ok_or(signature)?;
		self.aggregate_signature =
			Some(aggregate_signature.add_signature(&signature).map_err(|_| signature)?);
		Ok(())
	}

	/// Collects and returns the collection of fees fro for all withdrawals.
	pub fn get_fees(&self) -> Vec<Fees> {
		let mut fees = Vec::new();
		for withdrawal in &self.withdrawals {
			fees.push(Fees { asset: withdrawal.asset, amount: withdrawal.fees });
		}
		fees
	}

	pub fn add_auth_index(&mut self, index: usize) {
		set_bit_field(&mut self.bitflags, index);
	}

	/// Get set indexes.
	pub fn signed_auth_indexes(&self) -> Vec<usize> {
		return_set_bits(&self.bitflags)
	}

	/// Verifies the aggregate signature of the snapshot summary.
	pub fn verify(&self, public_keys: Vec<Public>) -> bool {
		let msg = self.sign_data();
		match self.aggregate_signature {
			None => false,
			Some(sig) => sig.verify(&public_keys, msg.as_ref()),
		}
	}

	/// Returns the data used for signing the snapshot summary.
	pub fn sign_data(&self) -> [u8; 32] {
		let data = (
			self.snapshot_id,
			self.state_root,
			self.state_change_id,
			self.worker_nonce,
			self.state_chunk_hashes.clone(),
			self.withdrawals.clone(),
		);

		sp_io::hashing::blake2_256(&data.encode())
	}
}

sp_api::decl_runtime_apis! {
	/// APIs necessary for Orderbook.
	pub trait ObApi
	{
		/// Return the current active Orderbook validator set.
		fn validator_set() -> ValidatorSet<crypto::AuthorityId>;

		/// Returns the latest Snapshot Summary.
		fn get_latest_snapshot() -> SnapshotSummary<AccountId>;

		/// Returns the snapshot summary for given snapshot id.
		fn get_snapshot_by_id(id: u64) -> Option<SnapshotSummary<AccountId>>;

		/// Return the ingress messages at the given block.
		fn ingress_messages(blk: polkadex_primitives::BlockNumber) -> Vec<polkadex_primitives::ingress::IngressMessages<AccountId>>;

		/// Submits the snapshot to runtime.
		#[allow(clippy::result_unit_err)]
		fn submit_snapshot(summary: SnapshotSummary<AccountId>) -> Result<(), ()>;

		/// Gets pending snapshot if any.
		fn pending_snapshot(auth: AuthorityId) -> Option<u64>;

		/// Returns all main account and corresponding proxies at this point in time.
		fn get_all_accounts_and_proxies() -> Vec<(AccountId,Vec<AccountId>)>;

		/// Returns Public Key of Whitelisted Orderbook Operator.
		fn get_orderbook_opearator_key() -> Option<sp_core::ecdsa::Public>;

		/// Returns snapshot generation intervals.
		fn get_snapshot_generation_intervals() -> (u64,BlockNumber);

		/// Returns last processed stid from last snapshot.
		fn get_last_accepted_worker_nonce() -> u64;

		/// Get all allow listed assets.
		fn get_allowlisted_assets() -> Vec<AssetId>;

		/// Reads the current trading pair configs.
		fn read_trading_pair_configs() -> Vec<(crate::types::TradingPair, TradingPairConfig)>;
	}
}
