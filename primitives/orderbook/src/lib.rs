#![cfg_attr(not(feature = "std"), no_std)]

mod bls;
#[cfg(feature = "std")]
pub mod types;
pub mod traits;

use crate::crypto::AuthorityId;
#[cfg(feature = "std")]
use crate::types::ObMessage;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use sp_core::ByteArray;
use sp_core::H256;
use sp_runtime::traits::IdentifyAccount;
use sp_std::vec::Vec;

/// Key type for BEEFY module.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"orbk");

/// BEEFY cryptographic types
///
/// This module basically introduces three crypto types:
/// - `crypto::Pair`
/// - `crypto::Public`
/// - `crypto::Signature`
///
/// Your code should use the above types as concrete types for all crypto related
/// functionality.
///
/// The current underlying crypto scheme used is ECDSA. This can be changed,
/// without affecting code restricted against the above listed crypto types.
pub mod crypto {
	use bls_primitives as BLS;
	use sp_application_crypto::app_crypto;
	app_crypto!(BLS, crate::KEY_TYPE);

	/// Identity of a BEEFY authority using ECDSA as its crypto.
	pub type AuthorityId = Public;

	/// Signature for a BEEFY authority using ECDSA as its crypto.
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
	validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	id: ValidatorSetId,
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return a validator set with the given validators and set id.
	pub fn new<I>(validators: I, id: ValidatorSetId) -> Option<Self>
	where
		I: IntoIterator<Item = AuthorityId>,
	{
		let validators: Vec<AuthorityId> = validators.into_iter().collect();
		if validators.is_empty() {
			// No validators; the set would be empty.
			None
		} else {
			Some(Self { validators, id })
		}
	}

	/// Return a reference to the vec of validators.
	pub fn validators(&self) -> &[AuthorityId] {
		&self.validators
	}

	/// Return the validator set id.
	pub fn id(&self) -> ValidatorSetId {
		self.id
	}

	/// Return the number of validators in the set.
	pub fn len(&self) -> usize {
		self.validators.len()
	}
}

/// The index of an authority.
pub type AuthorityIndex = u32;

#[derive(Copy, Clone, Encode, Decode)]
pub struct StidImportRequest {
	pub from: u64,
	pub to: u64,
}

#[derive(Clone, Encode, Decode, Default)]
#[cfg(feature = "std")]
pub struct StidImportResponse {
	pub messages: Vec<ObMessage>,
}

#[derive(Clone, Encode, Decode, Default, Debug, TypeInfo, PartialEq)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct SnapshotSummary {
	pub state_root: H256,
	pub state_change_id: u64,
	pub state_hash: H256,
	pub bitflags: Vec<u128>,
	pub aggregate_signature: Vec<u8>,
}

impl SnapshotSummary {
	pub fn verify(&self, public_keys: Vec<Vec<u8>>) -> bool {
		// TODO: change the required data types and implement
		// bls aggregate signature verification here
		true
	}
}

sp_api::decl_runtime_apis! {
	/// APIs necessary for Orderbook.
	pub trait ObApi
	{
		/// Return the current active Orderbook validator set
		fn validator_set() -> Option<ValidatorSet<crypto::AuthorityId>>;

		fn get_latest_snapshot() -> SnapshotSummary;
	}
}
