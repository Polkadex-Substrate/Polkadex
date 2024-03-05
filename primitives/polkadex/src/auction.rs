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

use codec::{Decode, Encode, MaxEncodedLen};
use frame_support::pallet_prelude::TypeInfo;
use frame_support::{Deserialize, Serialize};
use sp_std::collections::btree_map::BTreeMap;

#[derive(
    Clone, Encode, Decode, MaxEncodedLen, TypeInfo, Debug, PartialEq, Serialize, Deserialize,
)]
pub struct FeeDistribution<AccountId, BlockNo> {
    pub recipient_address: AccountId,
    pub auction_duration: BlockNo,
    pub burn_ration: u8,
}

#[derive(Clone, Encode, Decode, TypeInfo, Debug, PartialEq)]
pub struct AuctionInfo<AccountId, Balance> {
    pub fee_info: BTreeMap<u128, Balance>,
    pub highest_bidder: Option<AccountId>,
    pub highest_bid: Balance,
}

impl<AccountId, Balance: Default> Default for AuctionInfo<AccountId, Balance> {
    fn default() -> Self {
        Self {
            fee_info: BTreeMap::new(),
            highest_bidder: None,
            highest_bid: Balance::default(),
        }
    }
}
