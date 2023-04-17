#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{
	ocex::TradingPairConfig, withdrawal::Withdrawal, AccountId, AssetId, BlockNumber,
};
use primitive_types::H128;
use rust_decimal::Decimal;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_core::ByteArray;
use sp_core::H256;
use sp_runtime::traits::IdentifyAccount;
use sp_std::vec::Vec;

use bls_primitives::{Public, Signature};

#[cfg(feature = "std")]
use crate::types::ObMessage;
use crate::{
	crypto::AuthorityId,
	utils::{return_set_bits, set_bit_field},
};

pub mod constants;
pub mod types;
pub mod utils;

/// Key type for BEEFY module.
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

/// Authority set id starts with zero at genesis
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// A typedef for validator set id.
pub type ValidatorSetId = u64;

/// A set of BEEFY authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return a validator set with the given validators and set id.
	pub fn new<I>(validators: I, _id: ValidatorSetId) -> Option<Self>
	where
		I: IntoIterator<Item = AuthorityId>,
	{
		let validators: Vec<AuthorityId> = validators.into_iter().collect();
		if validators.is_empty() {
			// No validators; the set would be empty.
			None
		} else {
			Some(Self { validators })
		}
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

#[derive(Copy, Clone, Encode, Decode)]
pub struct StidImportRequest {
	pub from: u64,
	pub to: u64,
}

#[derive(Clone, Encode, Decode, TypeInfo, Debug, PartialEq)]
pub struct Fees {
	pub asset: AssetId,
	pub amount: Decimal,
}

#[derive(Clone, Encode, Decode, Default)]
#[cfg(feature = "std")]
pub struct StidImportResponse {
	pub messages: Vec<ObMessage>,
}

#[derive(Clone, Encode, Decode, Default, Debug, TypeInfo, PartialEq)]
pub struct SnapshotSummary {
	pub snapshot_id: u64,
	pub state_root: H256,
	pub worker_nonce: u64,
	pub state_change_id: u64,
	pub state_chunk_hashes: Vec<H128>,
	pub bitflags: Vec<u128>,
	pub withdrawals: Vec<Withdrawal<AccountId>>,
	pub aggregate_signature: Option<bls_primitives::Signature>,
}

impl SnapshotSummary {
	// Add a new signature to the snapshot summary
	pub fn add_signature(&mut self, signature: Signature) -> Result<(), Signature> {
		match bls_primitives::crypto::bls_ext::add_signature(
			&self.aggregate_signature.ok_or(signature)?,
			&signature,
		) {
			Ok(signature) => {
				self.aggregate_signature = Some(signature);
				Ok(())
			},
			Err(_) => Err(signature),
		}
	}

	pub fn get_fees(&self) -> Vec<Fees> {
		let mut fees = Vec::new();
		for withdrawal in &self.withdrawals {
			fees.push(Fees { asset: withdrawal.asset, amount: withdrawal.fees });
		}
		fees
	}

	pub fn add_auth_index(&mut self, index: u16) {
		set_bit_field(&mut self.bitflags, index);
	}

	// Get set indexes
	pub fn signed_auth_indexes(&self) -> Vec<u16> {
		return_set_bits(&self.bitflags)
	}

	// Verifies the aggregate signature of the snapshot summary
	pub fn verify(&self, public_keys: Vec<Public>) -> bool {
		let msg = self.sign_data();
		match self.aggregate_signature {
			None => false,
			Some(sig) =>
				bls_primitives::crypto::bls_ext::verify_aggregate(&public_keys, &msg, &sig),
		}
	}

	// Returns the data used for signing the snapshot summary
	pub fn sign_data(&self) -> [u8; 32] {
		let data = (
			self.snapshot_id,
			self.state_root,
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
		/// Return the current active Orderbook validator set
		fn validator_set() -> ValidatorSet<crypto::AuthorityId>;

		/// Returns the latest Snapshot Summary
		fn get_latest_snapshot() -> SnapshotSummary;

		/// Returns the snapshot summary for given snapshot id
		fn get_snapshot_by_id(id: u64) -> Option<SnapshotSummary>;

		/// Return the ingress messages at the given block
		fn ingress_messages() -> Vec<polkadex_primitives::ingress::IngressMessages<AccountId>>;

		/// Submits the snapshot to runtime
		#[allow(clippy::result_unit_err)]
		fn submit_snapshot(summary: SnapshotSummary) -> Result<(), ()>;

		/// Gets pending snapshot if any
		fn pending_snapshot() -> Option<u64>;

		/// Returns all main account and corresponding proxies at this point in time
		fn get_all_accounts_and_proxies() -> Vec<(AccountId,Vec<AccountId>)>;

		/// Returns Public Key of Whitelisted Orderbook Operator
		fn get_orderbook_opearator_key() -> Option<sp_core::ecdsa::Public>;

		/// Returns snapshot generation intervals
		fn get_snapshot_generation_intervals() -> (u64,BlockNumber);

		/// Get all allow listed assets
		fn get_allowlisted_assets() -> Vec<AssetId>;

		/// Reads the current trading pair configs
		fn read_trading_pair_configs() -> Vec<(crate::types::TradingPair, TradingPairConfig)>;
	}
}
