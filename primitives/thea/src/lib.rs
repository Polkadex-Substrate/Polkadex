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

//! # Thea Primitives.
//!
//! This crate contains common types and operations definition required for the `Thea` related
//! components.

#![feature(duration_constants)]
#![cfg_attr(not(feature = "std"), no_std)]

pub mod ethereum;
pub mod types;

pub use crate::types::Message;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::DispatchResult;
use sp_std::vec::Vec;

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

impl<AuthorityId> ValidatorSet<AuthorityId> {
	/// Returns a validator set with the given validators and set id.
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

	/// Returns a reference to the vec of validators.
	pub fn validators(&self) -> &[AuthorityId] {
		&self.validators
	}

	/// Returns the number of validators in the set.
	pub fn len(&self) -> usize {
		self.validators.len()
	}

	/// Return true if set is empty.
	pub fn is_empty(&self) -> bool {
		self.validators.is_empty()
	}
}

/// The index of an authority.
pub type AuthorityIndex = u32;

/// Network type.
pub type Network = u8;

/// Parachain Network ID
pub const PARACHAIN_NETWORK: Network = 1;

/// Ethereum Network ID
pub const ETHEREUM_NETWORK: Network = 1;

/// Native network id.
pub const NATIVE_NETWORK: Network = 0;

/// TTL of the cached message.
pub const MESSAGE_CACHE_DURATION_IN_SECS: u64 = 60;

/// Thea incoming message executor abstraction which should be implemented by the "Thea Executor".
pub trait TheaIncomingExecutor {
	fn execute_deposits(network: Network, deposits: Vec<u8>);
}

/// Thea outgoing message executor abstraction which should be implemented by the "Thea" pallet.
pub trait TheaOutgoingExecutor {
	fn execute_withdrawals(network: Network, withdrawals: Vec<u8>) -> DispatchResult;
}

impl TheaIncomingExecutor for () {
	fn execute_deposits(_network: Network, _deposits: Vec<u8>) {}
}
