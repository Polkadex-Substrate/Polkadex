use crate::mock::*;
use frame_support::{assert_ok, assert_noop};

use polkadex_primitives::assets::AssetId;
use sp_core::{H160};
use orml_traits::MultiCurrency;
use super::*;

#[test]
fn test_create_token() {

    // Register new account
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let new_balance: u128 = 500;
        // Chainsafe Asset
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        assert_eq!(PolkadexFungibleAssets::create_token(Origin::signed(alice.clone()), new_asset_chainsafe, new_balance), Ok(()));
        assert_eq!(OrmlToken::total_issuance(new_asset_chainsafe), 500u128);
        assert_eq!(OrmlToken::total_balance(new_asset_chainsafe, &alice), 500u128);
        // Snowfork Asset
        let new_asset_snofork: AssetId = AssetId::SNOWFORK(H160::from_low_u64_be(24));
        assert_eq!(PolkadexFungibleAssets::create_token(Origin::signed(alice.clone()), new_asset_snofork, new_balance), Ok(()));
        assert_eq!(OrmlToken::total_issuance(new_asset_chainsafe), 500u128);
        assert_eq!(OrmlToken::total_balance(new_asset_chainsafe, &alice), 500u128);
    });

    // Check for Error
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let new_balance: u128 = 500;
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        assert_eq!(PolkadexFungibleAssets::create_token(Origin::signed(alice.clone()), new_asset_chainsafe, new_balance), Ok(()));
        assert_noop!(PolkadexFungibleAssets::create_token(Origin::signed(alice.clone()), new_asset_chainsafe, new_balance), Error::<Test>::AssetIdAlreadyExists);
    });

    // Transfer of Balance
    new_tester().execute_with(|| {
        let alice: u64 = 1;
        let bob: u64 = 2;
        let new_balance: u128 = 500;
        let new_asset_chainsafe: AssetId = AssetId::CHAINSAFE(H160::from_low_u64_be(24));
        assert_eq!(PolkadexFungibleAssets::create_token(Origin::signed(alice.clone()), new_asset_chainsafe, new_balance), Ok(()));
        assert_eq!(OrmlToken::transfer(Origin::signed(alice.clone()),bob,new_asset_chainsafe, 200u128), Ok(().into()));
        assert_eq!(OrmlToken::total_balance(new_asset_chainsafe, &alice), 300u128);
        assert_eq!(OrmlToken::total_balance(new_asset_chainsafe, &bob), 200u128);

    });
}
