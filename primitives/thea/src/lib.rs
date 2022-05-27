// Copyright (C) 2020-2022 Polkadex OU
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// This is pallet is modified beefy-primitives from Parity Technologies (UK) Ltd.
#![cfg_attr(not(feature = "std"), no_std)]
// NOTE: needed to silence warnings about generated code in `decl_runtime_apis`
#![allow(clippy::too_many_arguments, clippy::unnecessary_mut_passed, clippy::redundant_slicing)]

pub mod constants;
pub mod inherents;
pub mod keygen;
pub mod payload;
pub mod runtime;
pub mod traits;

pub use constants::*;

use crate::{
	keygen::{KeygenRound, OfflineStageRound, SigningSessionPayload, TheaPayload},
	payload::*,
};
use codec::{Codec, Decode, Encode};
use polkadex_primitives::BlockNumber;
use scale_info::TypeInfo;
use sp_core::ecdsa::Public;
use sp_runtime::{
	traits::{IdentifyAccount, Verify},
	MultiSignature,
};
use sp_std::prelude::*;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// Key type for THEA module.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"thea");

/// Authority set id starts with zero at genesis
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// A typedef for validator set id.
pub type ValidatorSetId = u64;

#[derive(Decode, Encode, Debug, PartialEq, Copy, Clone, TypeInfo)]
pub enum KeyGenStage {
	/// Keygen is yet to start
	NotStarted,
	/// Keygen Failed
	Failed,
	/// Keygen completed successfully
	Completed,
}

impl Default for KeyGenStage {
	fn default() -> Self {
		KeyGenStage::NotStarted
	}
}

/// A set of THEA authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	pub id: ValidatorSetId,
	/// Thea ECDSA Public key
	pub public_key: Option<Public>,
}

impl ValidatorSet<AuthorityId> {
	pub fn new(validators: Vec<AuthorityId>, id: ValidatorSetId) -> Self {
		ValidatorSet { validators, id, public_key: None }
	}
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return an empty validator set with id of 0.
	pub fn empty() -> Self {
		Self { validators: Default::default(), id: Default::default(), public_key: None }
	}
}

/// THEA application-specific crypto types using sr25519.
pub mod crypto {
	use sp_application_crypto::{app_crypto, sr25519};

	app_crypto!(sr25519, crate::KEY_TYPE);
}

sp_application_crypto::with_pair! {
		/// A THEA authority keypair using sr25519 as its crypto.
		pub type AuthorityPair = crypto::Pair;
}

/// Identity of a THEA authority using sr25519 as its crypto.
pub type AuthorityId = crypto::Public;

/// Signature for a THEA authority using sr25519 as its crypto.
pub type AuthoritySignature = crypto::Signature;

/// The index of an authority.
pub type AuthorityIndex = u16;

/// Index of a Thea party
pub type PartyIndex = u16;

/// The `ConsensusEngineId` of THEA.
pub const THEA_ENGINE_ID: sp_runtime::ConsensusEngineId = *b"THEA";

/// A consensus log item for THEA.
#[derive(Decode, Encode, TypeInfo)]
pub enum ConsensusLog<AuthorityId: Codec> {
	/// The authorities have changed.
	#[codec(index = 1)]
	AuthoritiesChange(ValidatorSet<AuthorityId>),
	/// Disable the authority with given index.
	#[codec(index = 2)]
	OnDisabled(AuthorityIndex),
}

sp_api::decl_runtime_apis! {
	/// API necessary for THEA voters.
	pub trait TheaApi {
		/// Return the current active THEA validator set
		fn validator_set() -> ValidatorSet<AuthorityId>;
		/// Return the next active THEA validator set
		fn next_validator_set() -> ValidatorSet<AuthorityId>;
		/// Submit keygen message to on-chain
		fn submit_keygen_message(payload: TheaPayload<AuthorityId,KeygenRound,MsgLimit,MsgVecLimit>, signature: AuthoritySignature, rng: u64) -> Result<(),SigningError>;
		/// Submit offline message to on-chain
		fn submit_offline_message(payload: TheaPayload<AuthorityId,OfflineStageRound,MsgLimit,MsgVecLimit>, signature: AuthoritySignature, rng: u64, payload_array: &[u8; 32]) -> Result<(),SigningError>;
		/// Submit signing message to on-chain
		fn submit_signing_message(at: BlockNumber,payload: SigningSessionPayload<AuthorityId,PartialSignatureLimit,PartialSignatureVecLimit>, signature: AuthoritySignature, rng: u64) -> Result<(),SigningError>;
		/// Submit signed payload to on-chain
		fn submit_signed_payload(payload: SignedTheaPayload, rng: u64) -> Result<(),SigningError>;
		/// Get's other party keygen broadcast messages
		fn keygen_messages_api(party_idx: PartyIndex, round: KeygenRound) -> TheaPayload<AuthorityId, KeygenRound,MsgLimit,MsgVecLimit>;
		/// Get's other party offline broadcast messages
		fn offline_messages_api(party_idx: PartyIndex, round: OfflineStageRound, payload: &[u8; 32]) -> TheaPayload<AuthorityId, OfflineStageRound,MsgLimit,MsgVecLimit>;
		/// Get's other party signing broadcast messages
		fn signing_messages_api(at: BlockNumber) -> Vec<SigningSessionPayload<AuthorityId,PartialSignatureLimit,PartialSignatureVecLimit>>;
		/// Returns unsigned payload
		fn unsigned_payloads_api(at: BlockNumber) -> Vec<UnsignedTheaPayload>;
		/// Returns signed payload for given network
		fn signed_payloads_api(at: BlockNumber) -> Vec<SignedTheaPayload>;
		/// Return True if Validator Set Changes
		fn is_validator_set_changed() -> bool;
		/// Cleans Keygen and Offline messages On-Chain
		fn clean_keygen_messages(auth_idx: AuthorityIndex, signature: AuthoritySignature, rng: u64) -> Result<(),SigningError>;
		fn register_offence(signature: AuthoritySignature, offence: crate::keygen::OffenseReport<AuthorityId>) ->  Result<(),SigningError>;

	}
}
// Add fn Proto here

/// Possible Errors in On-chain signing
#[derive(Decode, Encode, TypeInfo, PartialEq, Debug)]
pub enum SigningError {
	OffchainUnsignedTxError,
}

impl core::fmt::Display for SigningError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "OffchainUnsignedTxError")
	}
}

//sp_api::decl_runtime_apis! {
//	pub trait TheaApi {
//		fn current_round_info(&self) -> FutureResult<()>;
//		fn offline_stage_info(&self) -> FutureResult<()>;
//	}
//}
