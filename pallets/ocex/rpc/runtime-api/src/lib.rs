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

use orderbook_primitives::ObCheckpointRaw;
use parity_scale_codec::Codec;
use polkadex_primitives::AssetId;
use rust_decimal::Decimal;
use sp_std::vec::Vec;

sp_api::decl_runtime_apis! {
	pub trait PolkadexOcexRuntimeApi<AccountId, Hash> where AccountId: Codec, Hash : Codec {
		fn get_ob_recover_state() ->  Result<Vec<u8>, sp_runtime::DispatchError>;
		// gets balance from given account of given asset
		fn get_balance(from: AccountId, of: AssetId) -> Result<Decimal, sp_runtime::DispatchError>;
		fn fetch_checkpoint() -> Result<ObCheckpointRaw, sp_runtime::DispatchError>;
	}
}
