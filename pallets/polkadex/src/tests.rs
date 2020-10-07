use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use frame_system::ensure_signed;
use sp_runtime::traits::Hash;

use crate::{Error, LinkedPriceLevel, mock::*, mock};
use crate::OrderType::{AskLimit, AskMarket, BidLimit, BidMarket};

use super::*;

const UNIT: u128 = 1_000_000_000_000;

fn setup_balances() {
    let alice: u64 = 1;
    let bob: u64 = 2;
    let options_alice = pallet_generic_asset::AssetOptions::<u128, u64> {
        initial_issuance: 1000 * UNIT,
        permissions: Default::default(),
    };
    // Creates first asset to alice's account
    assert_ok!(pallet_generic_asset::Module::<Test>::create_asset(None,Some(alice),options_alice));
    let options_bob = pallet_generic_asset::AssetOptions::<u128, u64> {
        initial_issuance: UNIT,
        permissions: Default::default(),
    };
    // Creates second asset to bob's account
    assert_ok!(pallet_generic_asset::Module::<Test>::create_asset(None,Some(bob),options_bob));
}

#[test]
fn setup_balances_test() {
    new_test_ext().execute_with(|| {
        setup_balances();
    });
}

#[test]
fn check_trading_engine() {
    new_test_ext().execute_with(|| {
        let alice: u64 = 1;
        let bob: u64 = 2;
        let trading_pair = create_trading_pair_id(&2, &1);
        // Creates two assets using Alice's and Bob's Accounts.
        setup_balances();
        assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
        // Place some random buy orders from Alice
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,820*UNIT,(2*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,800*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,850*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,840*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,900*UNIT,(1*UNIT)/10));
        // Place some random sell limit orders from Bob
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1075*UNIT,(2*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1100*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1060*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1040*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1000*UNIT,(1*UNIT)/10));
        // Place some random market orders
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,(UNIT/100)*5,0));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskMarket,trading_pair,0,(UNIT/1000)*5));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,(UNIT/1000)*16,0));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskMarket,trading_pair,0,(UNIT/1000)*16));
        // Read the block chain state for verifying
        // Balances of Token #1 for Alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), ((UNIT / 1000) * 496934));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), ((UNIT / 10) * 4841));
        // Balances of Token #2 for Alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), ((UNIT / 1000000) * 21066));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), 0);
        // Balances of Token #1 for Bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob), ((UNIT / 1000) * 18966));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob), 0);
        // Balances of Token #2 for Bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), ((UNIT / 1000) * 379));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), ((UNIT / 1000000) * 599934));
    });
}

#[test]
fn correct_error_for_low_price_or_quantity() {
    new_test_ext().execute_with(|| {
        let alice: u64 = 1;
        let trading_pair = create_trading_pair_id(&2, &1);
        // Creates two assets using Alice's and Bob's Accounts.
        setup_balances();
        assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
        // Place orders with too low price and quantity values
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,UNIT/1000000,(2*UNIT)/10),Error::<Test>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),AskLimit,trading_pair,UNIT,UNIT/1000000),Error::<Test>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,0,0),Error::<Test>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),AskMarket,trading_pair,0,0),Error::<Test>::PriceOrQuantityTooLow);

        // Trying to Overflow
        let price = UNIT;
        let quantity = 10000000000000000000 * UNIT;
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,price,quantity),Error::<Test>::OverFlowError);
    });
}

#[test]
fn correct_error_for_invalid_trading_pair() {
    new_test_ext().execute_with(|| {
        let alice: u64 = 1;
        let trading_pair = create_trading_pair_id(&2, &1);
        // Creates two assets using Alice's and Bob's Accounts.
        setup_balances();
        // assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1)); <-- Not Registering trading pair
        // Place orders with too low price and quantity values
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,UNIT,UNIT),Error::<Test>::InvalidTradingPair);
    });
}

#[test]
fn correct_error_for_insufficient_balance_to_register_trading_pair() {
    new_test_ext().execute_with(|| {
        let charlie: u64 = 3;
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(charlie),2,1),Error::<Test>::InsufficientAssetBalance);
    });
}

#[test]
fn correct_error_for_registering_same_trading_pair() {
    new_test_ext().execute_with(|| {
        let alice: u64 = 1;
        let bob: u64 = 2;
        assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(bob),2,1),Error::<Test>::TradingPairIDExists);
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(bob),1,2),Error::<Test>::TradingPairIDExists);
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(bob),1,1),Error::<Test>::SameAssetIdsError);
    });
}

#[test]
fn check_cancel_order() {
    new_test_ext().execute_with(|| {
        let alice: u64 = 1;
        let trading_pair = create_trading_pair_id(&2, &1);
        setup_balances();
        assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
        // Place some random buy orders from Alice
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,UNIT,UNIT));
        let price = DEXModule::convert_balance_to_fixed_u128(UNIT).unwrap();
        let mut pricelevel: LinkedPriceLevel<Test> = <PriceLevels<Test>>::get(trading_pair, price);
        assert_eq!(pricelevel.orders.len(), 1);
        let order = pricelevel.orders.pop_front().unwrap();
        assert_ok!(DEXModule::cancel_order(Origin::signed(alice),order.id,trading_pair,UNIT));
        pricelevel = <PriceLevels<Test>>::get(trading_pair, price);
        assert_eq!(pricelevel.orders.len(), 0);
    });
}


fn create_trading_pair_id(quote_asset_id: &u32, base_asset_id: &u32) -> <mock::Test as frame_system::Trait>::Hash {
    (quote_asset_id, base_asset_id).using_encoded(<Test as frame_system::Trait>::Hashing::hash)
}


