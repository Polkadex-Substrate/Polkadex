// Copyright (C) 2020-2021 Polkadex OU
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

use codec::{Decode, Encode};

use scale_info::TypeInfo;


use sp_std::prelude::*;


/// Key type for THEA module.
pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

/// Authority set id starts with zero at genesis
pub const GENESIS_AUTHORITY_SET_ID: u64 = 0;

/// A typedef for validator set id.
pub type ValidatorSetId = u64;



/// A set of OCEX authorities, a.k.a. validators.
#[derive(Decode, Encode, Debug, PartialEq, Clone, TypeInfo)]
pub struct ValidatorSet<AuthorityId> {
	/// Public keys of the validator set elements
	pub validators: Vec<AuthorityId>,
	/// Identifier of the validator set
	pub id: ValidatorSetId,
}

impl ValidatorSet<AuthorityId> {
	pub fn new(validators: Vec<AuthorityId>, id: ValidatorSetId) -> Self {
		ValidatorSet { validators, id }
	}
}

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Return an empty validator set with id of 0.
	pub fn empty() -> Self {
		Self { validators: Default::default(), id: Default::default() }
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


sp_api::decl_runtime_apis! {
	/// API necessary for OCEX validators.
	pub trait OcexApi {
		// Get list of enclaves waiting for verification by this verifier
		fn unverified_reports(verifier: AuthorityId) -> Vec<bool>;
	}
}

