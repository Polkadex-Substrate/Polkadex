// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
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
use codec::Codec;
use codec::Decode;
use polkadex_primitives::assets::AssetId;
use sp_std::vec::Vec;
use polkadex_primitives::withdrawal::Withdrawal;
use sp_runtime::traits::Zero;
sp_api::decl_runtime_apis! {
	pub trait PolkadexOcexRuntimeApi<AccountId,Hash,Balance> where AccountId: Codec, Hash : Codec, Balance: Zero + Clone + Codec{
        fn return_withdrawals(snapshot_ids: Vec<u32>,account: AccountId) -> Vec<Withdrawal<AccountId, Balance>>;
	}
}
