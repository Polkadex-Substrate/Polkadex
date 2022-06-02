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
use pallet_polkadex_ido_primitives::*;
use sp_std::vec::{Vec};
use polkadex_primitives::assets::AssetId;
sp_api::decl_runtime_apis! {
    pub trait PolkadexIdoRuntimeApi<AccountId,Hash> where AccountId: Codec, Hash : Codec{
        fn rounds_by_investor(account : AccountId) -> Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>;
        fn rounds_by_creator(account : AccountId) -> Vec<(Hash, FundingRoundWithPrimitives<AccountId>)> ;
        fn active_rounds() -> Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>;
        fn votes_stat(round_id: Hash) -> VoteStat;
        fn account_balances(assets : Vec<u128>, account_id : AccountId) ->  Vec<u128>;
    }
}
