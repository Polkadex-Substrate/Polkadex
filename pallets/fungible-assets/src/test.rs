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

#[test]
fn test_create_token() {
    // Register new account
    new_tester().execute_with(|| {
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
        assert_eq!(OrmlToken::total_issuance(new_asset_chainsafe), 500u128);
        assert_eq!(
            OrmlToken::total_balance(new_asset_chainsafe, &alice),
            500u128
        );

        assert_eq!(OrmlToken::total_issuance(new_asset_chainsafe), 500u128);
        assert_eq!(
            OrmlToken::total_balance(new_asset_chainsafe, &alice),
            500u128
        );
    });
    // Check for Error
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let new_balance: u128 = 500;
        let existential_deposit: u128 = 1;
        let mint_account = Some(2u64);
        let burn_account = Some(3u64);
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
        assert_noop!(
            PolkadexFungibleAssets::create_token(
                Origin::signed(alice.clone()),
                new_asset_chainsafe,
                new_balance,
                mint_account,
                burn_account,
                existential_deposit
            ),
            Error::<Test>::AssetIdAlreadyExists
        );
    });

    // Transfer of Balance
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let bob: u64 = 2;
        let new_balance: u128 = 500;
        let existential_deposit: u128 = 1;
        let mint_account = Some(2u64);
        let burn_account = Some(3u64);
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
        assert_eq!(
            OrmlToken::transfer(
                Origin::signed(alice.clone()),
                bob,
                new_asset_chainsafe,
                200u128
            ),
            Ok(().into())
        );
        assert_eq!(
            OrmlToken::total_balance(new_asset_chainsafe, &alice),
            300u128
        );
        assert_eq!(OrmlToken::total_balance(new_asset_chainsafe, &bob), 200u128);
    });
}

#[test]
fn test_mint_fungible() {
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let mint_account: u64 = 2;
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        setup_for_mint();
        assert_ok!(PolkadexFungibleAssets::mint_fungible(
            Origin::signed(mint_account.clone()),
            alice,
            new_asset_chainsafe,
            20u128
        ));
        assert_eq!(
            OrmlToken::total_balance(new_asset_chainsafe, &alice),
            520u128
        );
        assert_eq!(OrmlToken::total_issuance(new_asset_chainsafe), 520u128);
    });

    // Check Error
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let mint_account: u64 = 5;
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        setup_for_mint();
        assert_noop!(
            PolkadexFungibleAssets::mint_fungible(
                Origin::signed(mint_account.clone()),
                alice,
                new_asset_chainsafe,
                20u128
            ),
            Error::<Test>::NoPermissionToMint
        );
        let wrong_asset_id: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(28));
        let mint_account: u64 = 2;
        assert_noop!(
            PolkadexFungibleAssets::mint_fungible(
                Origin::signed(mint_account.clone()),
                alice,
                wrong_asset_id,
                20u128
            ),
            Error::<Test>::AssetIdNotExists
        );
    });
}

fn setup_for_burn() {
    let alice: u64 = 1;
    let new_balance: u128 = 500;
    let existential_deposit: u128 = 1;
    let mint_account = Some(1u64);
    let burn_account = Some(1u64);
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

#[test]
fn test_burn_fungible() {
    new_tester().execute_with(|| {
        let burn_account: u64 = 1;
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        setup_for_burn();
        assert_ok!(PolkadexFungibleAssets::burn_fungible(
            Origin::signed(burn_account.clone()),
            new_asset_chainsafe,
            20u128
        ));
        assert_eq!(
            OrmlToken::total_balance(new_asset_chainsafe, &burn_account),
            480u128
        );
        assert_eq!(OrmlToken::total_issuance(new_asset_chainsafe), 480u128);
    });

    // Check Error
    new_tester().execute_with(|| {
        let burn_account: u64 = 5;
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        setup_for_mint();
        assert_noop!(
            PolkadexFungibleAssets::burn_fungible(
                Origin::signed(burn_account.clone()),
                new_asset_chainsafe,
                20u128
            ),
            Error::<Test>::NoPermissionToBurn
        );
        let wrong_asset_id: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(28));
        let burn_account: u64 = 3;
        assert_noop!(
            PolkadexFungibleAssets::burn_fungible(
                Origin::signed(burn_account.clone()),
                wrong_asset_id,
                20u128
            ),
            Error::<Test>::AssetIdNotExists
        );
    });
}

#[test]
fn test_set_metadata_fungible() {
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let new_balance: u128 = 500;
        let existential_deposit: u128 = 1;
        let mint_account = Some(2u64);
        let burn_account = Some(3u64);
        let meta_data: AssetMetadata = AssetMetadata {
            name: "test".encode(),
            team: "".encode(),
            website: "".encode(),
        };
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
        assert_eq!(InfoAsset::<Test>::contains_key(new_asset_chainsafe), true);
        assert_eq!(
            PolkadexFungibleAssets::set_metadata_fungible(
                Origin::signed(alice.clone()),
                new_asset_chainsafe,
                meta_data
            ),
            Ok(())
        );
    });

    // Check for Error
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let new_balance: u128 = 500;
        let existential_deposit: u128 = 1;
        let mint_account = Some(2u64);
        let burn_account = Some(3u64);
        let meta_data: AssetMetadata = AssetMetadata {
            name: "test".encode(),
            team: "".encode(),
            website: "".encode(),
        };
        let bob: u64 = 2;
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
        assert_noop!(
            PolkadexFungibleAssets::set_metadata_fungible(
                Origin::signed(bob.clone()),
                new_asset_chainsafe,
                meta_data
            ),
            Error::<Test>::NotTheOwner
        );
    });
}

#[test]
fn test_attest_token() {
    new_tester().execute_with(|| {
        let alice: u64 = 6;
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
        assert_eq!(
            PolkadexFungibleAssets::attest_token(
                Origin::signed(alice.clone()),
                new_asset_chainsafe
            ),
            Ok(())
        );
        assert_eq!(
            InfoAsset::<Test>::get(new_asset_chainsafe).is_verified,
            true
        );
    });
}

#[test]
fn test_modify_token_deposit_amount() {
    new_tester().execute_with(|| {
        let alice: u64 = 6;
        let token: u128 = 123;
        assert_eq!(
            PolkadexFungibleAssets::modify_token_deposit_amount(
                Origin::signed(alice.clone()),
                token
            ),
            Ok(())
        );
        assert_eq!(FixedPDXAmount::<Test>::get(), 123);
    });
}