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

#![cfg_attr(not(feature = "std"), no_std)]

use parity_scale_codec::{Codec, Decode};
use polkadex_primitives::AssetId;
use orderbook_primitives::ObCheckpointRaw;
use rust_decimal::Decimal;
use sp_std::{collections::btree_map::BTreeMap, vec::Vec};

sp_api::decl_runtime_apis! {
	pub trait PolkadexOcexRuntimeApi<AccountId, Hash> where AccountId: Codec, Hash : Codec, BTreeMap<AccountId,Vec<AccountId>>: Decode {
		// Returns all on-chain registered main accounts and it's proxies
		fn get_main_accounts() -> BTreeMap<AccountId,Vec<AccountId>>;
		// gets the finalized state to recover from
		fn get_ob_recover_state() ->  Result<Vec<u8>, sp_runtime::DispatchError>;
		// gets balance from given account of given asset
		fn get_balance(from: AccountId, of: AssetId) -> Result<Decimal, sp_runtime::DispatchError>;
		// gets the latest checkpoint from the offchain State
		fn fetch_checkpoint() -> Result<ObCheckpointRaw, sp_runtime::DispatchError>;
		// Returns the asset inventory deviation in the offchain State
		fn calculate_inventory_deviation(offchain_inventory: BTreeMap<AssetId,Decimal>, last_processed_block: u32) -> Result<BTreeMap<AssetId,Decimal>, sp_runtime::DispatchError>;
	}
}
