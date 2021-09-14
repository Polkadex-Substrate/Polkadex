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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::Decode;
use cid::Cid;
use sp_std::vec::Vec;
use polkadex_primitives::AccountId;

pub mod inherents;

sp_api::decl_runtime_apis! {
	pub trait IpfsApi
	{
        /// Provides the cid account
        fn get_latest_cid() -> Option<Cid>;
        /// True if the exchange is operational
        fn check_emergency_closure() -> bool;
        /// Approved CID
        fn get_approved_cid() -> Option<Cid>;
        /// Get all user claims
        fn collect_user_claims() -> Vec<AccountId>;
        /// Get all enclave multiaddrs
        fn collect_enclave_multiaddrs() -> Vec<(AccountId,Vec<Vec<u8>>)>;
	}
}
