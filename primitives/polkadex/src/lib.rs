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

//! # Polkadex Primitives.
//!
//! Low-level types used throughout the Substrate code.

#![cfg_attr(not(feature = "std"), no_std)]

pub mod assets;
pub mod auction;
pub mod fees;
pub mod ingress;
pub mod rewards;
pub mod withdrawal;

pub use frame_support::storage::bounded_vec::BoundedVec;

use codec::{Decode, Encode};
use frame_support::traits::Get;
use scale_info::TypeInfo;
#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_runtime::{
	generic,
	traits::{BlakeTwo256, IdentifyAccount, Verify},
	MultiSignature, OpaqueExtrinsic,
};

// reexports:
pub use assets::*;

/// Balance unit.
pub const UNIT_BALANCE: u128 = 1_000_000_000_000;

/// Native "Polkadex" asset id.
pub const POLKADEX_NATIVE_ASSET_ID: u128 = 0;

/// An index to a block.
pub type BlockNumber = u32;

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = MultiSignature;

/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

/// The type for looking up accounts. We don't expect more than 4 billion of them.
pub type AccountIndex = u32;

/// Balance of an account.
pub type Balance = u128;

/// Type used for expressing timestamp.
pub type Moment = u64;

/// Index of a transaction in the chain.
pub type Index = u32;

/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;

/// Index of a transaction in the chain.
pub type Nonce = u32;

/// A timestamp: milliseconds since the unix epoch.
/// `u64` is enough to represent a duration of half a billion years, when the
/// time scale is milliseconds.
pub type Timestamp = u64;

/// Digest item type.
pub type DigestItem = generic::DigestItem;
/// Header type.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type.
pub type Block = generic::Block<Header, OpaqueExtrinsic>;
/// Block ID.
pub type BlockId = generic::BlockId<Block>;

/// Defines a limit of the proxy accounts per main account.
#[derive(Debug, Clone, Copy, PartialEq, TypeInfo, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct ProxyLimit;
impl Get<u32> for ProxyLimit {
	/// Accessor to the proxy accounts amount limit amount.
	fn get() -> u32 {
		3
	}
}

/// Defines a limit of the assets per main account.
#[derive(Debug, Clone, Copy, PartialEq, TypeInfo, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub struct AssetsLimit;
impl Get<u32> for AssetsLimit {
	/// Accessor to the assets amount limit amount.
	fn get() -> u32 {
		1000
	}
}
