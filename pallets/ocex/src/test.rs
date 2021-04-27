// This file is part of Polkadex.

// Copyright (C) 2020-2021 Polkadex oü.
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

use crate::mock::*;
use frame_support::{assert_noop, assert_ok};

use super::*;
use orml_traits::MultiCurrency;
use polkadex_primitives::assets::AssetId;
use sp_core::H160;

fn setup_for_mint() {
    let alice: u64 = 1;
    let new_balance: u128 = 500;
    let existential_deposit: u128 = 1;
    let mint_account = Some(2u64);
    let burn_account = Some(3u64);
    // Chainsafe Asset
    let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
    assert_eq!(
        PolkadexFungibleAssets::create_token(
            Origin::signed(alice.clone()),
            new_asset_chainsafe,
            new_balance,
            mint_account,
            burn_account,
            existential_deposit
        ),
        Ok(())
    );
}
