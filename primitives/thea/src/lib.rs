#![feature(duration_constants)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod types;

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;
/// Key type for Orderbook module.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"thea");
pub use crate::{
	crypto::{AuthorityId, AuthoritySignature},
	types::Message,
};
use sp_application_crypto::ByteArray;
use sp_runtime::DispatchResult;

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
use sp_runtime::traits::IdentifyAccount;

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

pub const THEA_WORKER_PREFIX: &[u8; 18] = b"Thea Worker Prefix";

/// A typedef for validator set id.
pub type ValidatorSetId = u64;

/// A set of Orderbook authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct ValidatorSet<AuthorityId> {
	/// Validator Set id
	pub set_id: ValidatorSetId,
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
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
			Some(Self { set_id: id, validators })
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

/// Network type
pub type Network = u8;

pub const NATIVE_NETWORK: Network = 0;

pub const MESSAGE_CACHE_DURATION_IN_SECS: u64 = 60;

sp_api::decl_runtime_apis! {
	/// APIs necessary for Thea.
	pub trait TheaApi
	{
		/// Return the current active Thea validator set for all networks
		fn full_validator_set() -> Option<ValidatorSet<AuthorityId>>;
		/// Return the current active Thea validator set
		fn validator_set(network: Network) -> Option<ValidatorSet<AuthorityId>>;
		/// Returns the outgoing message for given network and blk
		fn outgoing_messages(network: Network, nonce: u64) -> Option<Message>;
		/// Get Thea network associated with Validator
		fn network(auth: AuthorityId) -> Option<Network>;
		/// Incoming messages
		#[allow(clippy::result_unit_err)]
		fn incoming_message(message: Message, bitmap: Vec<u128>, signature: AuthoritySignature) -> Result<(),()>;
		/// Get last processed nonce for a given network
		fn get_last_processed_nonce(network: Network) -> u64;
	}
}

/// This is implemented by TheaExecutor by zK
pub trait TheaIncomingExecutor {
	fn execute_deposits(network: Network, deposits: Vec<u8>);
}
// This is implemented by Thea pallet by gj.
pub trait TheaOutgoingExecutor {
	fn execute_withdrawals(network: Network, withdrawals: Vec<u8>) -> DispatchResult;
}

impl TheaIncomingExecutor for () {
	fn execute_deposits(_network: Network, _deposits: Vec<u8>) {}
}
