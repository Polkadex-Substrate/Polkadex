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
use frame_support::assert_noop;

use super::*;
use polkadex_primitives::assets::AssetId;

#[test]
fn test_register_investor() {
    // Register new account
    ExtBuilder::default()
        .build().execute_with(|| {
        assert_eq!(
            PolkadexIdo::register_investor(
                Origin::signed(ALICE.clone())
            ),
            Ok(())
        );
        assert_noop!(
            PolkadexIdo::register_investor(
                Origin::signed(ALICE.clone())
            ),
            Error::<Test>::InvestorAlreadyRegistered
        );
    });
}

#[test]
fn test_attest_investor() {
    let bob: u64 = 6;
    ExtBuilder::default()
        .build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::attest_investor(
                Origin::signed(bob),
                ALICE.clone(),
                KYCStatus::Tier1
            ),
            Error::<Test>::InvestorDoesNotExist
        );
        assert_eq!(
            PolkadexIdo::register_investor(
                Origin::signed(ALICE.clone())
            ),
            Ok(())
        );
        assert_eq!(
            PolkadexIdo::attest_investor(
                Origin::signed(bob),
                ALICE.clone(),
                KYCStatus::Tier1
            ),
            Ok(())
        );
    });
}

#[test]
fn test_register_round() {
    let balance: Balance = 100;
    let block_num = 3;
    ExtBuilder::default()
        .build().execute_with(|| {
        assert_noop!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                block_num,
                balance,
                balance,
                balance,
                balance,
                block_num
            ),
            Error::<Test>::InvestorDoesNotExist
        );
        assert_eq!(
            PolkadexIdo::register_investor(
                Origin::signed(ALICE.clone())
            ),
            Ok(())
        );
        assert_eq!(
            PolkadexIdo::register_round(
                Origin::signed(ALICE.clone()),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                block_num,
                balance,
                balance,
                balance,
                balance,
                block_num
            ),
            Ok(())
        );

    });
}

