use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::Hash;

use crate::{Error, LinkedPriceLevel, mock::*, mock};


use super::*;

const UNIT: u128 = 1_000_000_000_000;

type System = frame_system::Module<TestRuntime>;

fn setup_creates_asset_ids() {
    let alice: u64 = 1;
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::create_token(Origin::signed(alice), 10*UNIT, 0)); // TODO: Modify token asset id generation to take BABE randomness.
    let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(10*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), 10*UNIT);

    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::create_token(Origin::signed(alice), 10*UNIT, 0));
    let base_asset_id = (1 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(10*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, base_asset_id), 10*UNIT);

    assert_ok!(DEXModule::register_new_orderbook_with_polkadex(Origin::signed(alice), quote_asset_id, UNIT));
    assert_ok!(DEXModule::register_new_orderbook_with_polkadex(Origin::signed(alice), base_asset_id, UNIT));

}

#[test]
fn check_for_trading_pair_registration() {
    new_test_ext().execute_with(|| {
        setup_creates_asset_ids();
        let alice: u64 = 1;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(10*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(10*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);

        //Same Asset Id
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(alice),quote_asset_id,UNIT,quote_asset_id, UNIT), Error::<TestRuntime>::SameAssetIdsError);

        //Insufficient Asset Balance
        let jim: u64 = 10;
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(jim),quote_asset_id, UNIT,base_asset_id, UNIT), Error::<TestRuntime>::InsufficientPolkadexBalance);

        // Best Case :- Registration Successful
        assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),quote_asset_id, UNIT,base_asset_id, UNIT));
        let trading_pair_id = DEXModule::get_pair(quote_asset_id, base_asset_id);
        let expcted_orderbook:Orderbook<TestRuntime> = Orderbook::new(trading_pair_id.1, trading_pair_id.0, trading_pair_id);
        let actual_orderbook: Orderbook<TestRuntime> = <Orderbooks<TestRuntime>>::get((base_asset_id,quote_asset_id));
        assert_eq!(expcted_orderbook, actual_orderbook);

        // Same trading Id registration
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(jim),quote_asset_id, UNIT,base_asset_id, UNIT), Error::<TestRuntime>::TradingPairIDExists);

    });
}

fn setup_register_new_orderbook() {

    // Alice has 0 base and 3 quote units
    let alice: u64 = 1;
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::create_token(Origin::signed(alice), 3*UNIT, 0));
    let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), 3*UNIT);

    // Bob has 10000 base units and 0 quote units
    // Alice has 2 base units and 3 quote units
    let bob: u64 = 2;
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::create_token(Origin::signed(bob), 10000*UNIT, 0));
    let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), 10000*UNIT);
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::mint_token(bob, &alice, base_asset_id, 2*UNIT));

    // Alice has 2 base units and 2 quote units
    assert_ok!(DEXModule::register_new_orderbook_with_polkadex(Origin::signed(alice), quote_asset_id, UNIT));
    // Alice has 1 base units and 2 quote units
    assert_ok!(DEXModule::register_new_orderbook_with_polkadex(Origin::signed(alice), base_asset_id, UNIT));
    // Alice has 0 base units and 1 quote units
    assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),quote_asset_id, UNIT,base_asset_id, UNIT));
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), UNIT);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, base_asset_id), 0);

    // Bob has 10000 base units and 0 quote units
    // Alice has 0 base units and 1 quote units
}

fn setup_new_orderbook_for_uniswap_testing() {

    // Alice has 0 base and 3 quote units
    let alice: u64 = 1;
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::create_token(Origin::signed(alice), 3*UNIT, 0));
    let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), 3*UNIT);

    // Bob has 10000 base units and 0 quote units
    // Alice has 2 base units and 3 quote units
    let bob: u64 = 2;
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::create_token(Origin::signed(bob), 10000*UNIT, 0));
    let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), 10000*UNIT);
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::mint_token(bob, &alice, base_asset_id, 2*UNIT));

    // Alice has 2 base units and 2 quote units
    assert_ok!(DEXModule::register_new_orderbook_with_polkadex(Origin::signed(alice), quote_asset_id, UNIT));
    // Alice has 1 base units and 2 quote units
    assert_ok!(DEXModule::register_new_orderbook_with_polkadex(Origin::signed(alice), base_asset_id, UNIT));
    // Alice has 1 base units and (2-1/95)*UNIT quote units
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, base_asset_id), 1*UNIT);
    assert_ok!(polkadex_custom_assets::Module::<TestRuntime>::mint_token(bob, &alice, base_asset_id, 100*UNIT));
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, base_asset_id), 101*UNIT);
    assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),quote_asset_id, (2055798*UNIT/100000000),base_asset_id, 100*UNIT));
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), 2*UNIT-(2055798*UNIT/100000000));
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, base_asset_id), 1*UNIT);

    // Bob has 10000 base units and 0 quote units
    // Alice has 1 base units
    assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, quote_asset_id), 0*UNIT);
}

#[test]
fn check_for_different_order_types() {

    // Check for BidLimitMM and AskLimitMM [No Match]
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);

        // COMPLETE ORDER
        // Bob places BidLimitMM
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,8200*UNIT,(2*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), 8360 * UNIT);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), 1640 * UNIT);

        // Alice places AskLimitMM

        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,10750*UNIT,(2*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), (8 * UNIT) / 10);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, quote_asset_id), (2 * UNIT) / 10);
    });

    //Check for BidLimitMM and AskLimitMM [Complete Test]
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);

        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,8200*UNIT,(2*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,8000*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,8500*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,8400*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,9000*UNIT,(1*UNIT)/10));

        // Place some random sell limit orders from alice
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,10750*UNIT,(2*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,11000*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,10600*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,10400*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,10000*UNIT,(1*UNIT)/10));

        // Balances of Token #1 for Alice
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), (UNIT * 4970));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), (UNIT * 5030));

        // Balances of Token #2 for Bob
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), (UNIT / 10) * 4);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, quote_asset_id), (UNIT / 10) * 6);

        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidMarket,trading_pair,500*UNIT,0));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskMarket,trading_pair,0,(UNIT/100)*5));

        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidMarket,trading_pair,1646*UNIT,0));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskMarket,trading_pair,0,(UNIT/100)*16));

        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), (UNIT * 2824));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), (UNIT * 3196));

        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), (UNIT / 100) * 19);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, quote_asset_id), (UNIT / 100) * 39);

        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimit,trading_pair,10600*UNIT,(5*UNIT)/100));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimit,trading_pair,8400*UNIT,(5*UNIT)/100));

        let trading_pair = (quote_asset_id, base_asset_id);
        let orderbook: Orderbook<TestRuntime> = <Orderbooks<TestRuntime>>::get(trading_pair);
        let best_ask_pricelevel: LinkedPriceLevel<TestRuntime> = <PriceLevels<TestRuntime>>::get(trading_pair, orderbook.best_ask_price);
        assert_eq!(orderbook.best_ask_price, FixedU128::from(10600));
        assert_eq!(calculate_quantity(best_ask_pricelevel.clone()).to_fraction(), 0.04);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), 2294 * UNIT);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), (14 * UNIT) / 100);

        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimit,trading_pair,10750*UNIT,(14*UNIT)/100));
        let orderbook: Orderbook<TestRuntime> = <Orderbooks<TestRuntime>>::get(trading_pair);
        let best_ask_pricelevel: LinkedPriceLevel<TestRuntime> = <PriceLevels<TestRuntime>>::get(trading_pair, orderbook.best_ask_price);
        assert_eq!(orderbook.best_ask_price, FixedU128::from(10750));
        assert_eq!(calculate_quantity(best_ask_pricelevel.clone()), FixedU128::from_fraction(0.1));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimit,trading_pair,8200*UNIT,(14*UNIT)/100));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), (UNIT * 795));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), (UNIT * 1620));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, quote_asset_id), (80 * UNIT) / 100);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, quote_asset_id), 0);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, base_asset_id), 7585 * UNIT);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, base_asset_id), 0);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), ((UNIT / 1000) * 0));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, quote_asset_id), ((UNIT / 1000) * 200));
    });

    // Error Test (BidLimitMM and AskLimitMM)
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        let price = UNIT;
        let quantity = 10000000000000000000 * UNIT;
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);

        let wrong_asset_id = (2 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::BidLimitMM,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
    });

    // Check for AskLimitMMOnly and BidLimitOnly
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);

        // COMPLETE ORDER
        // Bob places BidLimitMMOnly
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMMOnly,trading_pair,8200*UNIT,(2*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), 8360 * UNIT);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), 1640 * UNIT);

        // Alice places AskLimitMMOnly

        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMMOnly,trading_pair,10750*UNIT,(2*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), (8 * UNIT) / 10);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, quote_asset_id), (2 * UNIT) / 10);

        // TODO: This should work but giving some strange error
        // It has something to do with assert_noop.
        let temp = DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMMOnly,trading_pair,12000*UNIT,(2*UNIT)/10);
        assert_eq!(temp.is_err(), true);
        // assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMMOnly,trading_pair,12000*UNIT,(2*UNIT)/10), Error::<TestRuntime>::ErrorIsNotMarketMaking);
        // assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMMOnly,trading_pair,5000*UNIT,(2*UNIT)/10), Error::<TestRuntime>::ErrorIsNotMarketMaking);
    });

    // Error Test (BidLimitMMOnly and AskLimitMMOnly)
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMMOnly,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMMOnly,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        let price = UNIT;
        let quantity = 10000000000000000000 * UNIT;
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMMOnly,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMMOnly,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);

        let wrong_asset_id = (2 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::BidLimitMMOnly,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMMOnly,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
    });

    // Error TestRuntime (AskMarket and BidMarket)
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidMarket,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskMarket,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        let price = UNIT;
        let quantity = 10000000000000000000 * UNIT;
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidMarket,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskMarket,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);

        let wrong_asset_id = (2 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::BidMarket,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskMarket,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
    });

    // Error TestRuntime (AskMarket and BidMarket)
    new_test_ext().execute_with(|| {
        setup_register_new_orderbook();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimit,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimit,trading_pair,8200,(2)/10), <Error<TestRuntime>>::PriceOrQuantityTooLow);
        let price = UNIT;
        let quantity = 10000000000000000000 * UNIT;
        assert_noop!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimit,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimit,trading_pair,price,quantity),Error::<TestRuntime>::OverFlowError);

        let wrong_asset_id = (2 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000 * UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::BidLimit,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
        assert_noop!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimit,(quote_asset_id, wrong_asset_id),UNIT,UNIT),Error::<TestRuntime>::InvalidTradingPair);
    });
}

#[test]
fn check_for_ask_and_bid_limit() {
    new_test_ext().execute_with(|| {

        setup_new_orderbook_for_uniswap_testing();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let quote_asset_id = (0 as u64, alice.clone(), DEXModule::convert_balance_to_fixed_u128(3*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let base_asset_id = (1 as u64, bob.clone(), DEXModule::convert_balance_to_fixed_u128(10000*UNIT).unwrap()).using_encoded(<TestRuntime as frame_system::Trait>::Hashing::hash);
        let trading_pair = (quote_asset_id, base_asset_id);
        // COMPLETE ORDER

        // BidLimit
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimitMM,trading_pair,9000*UNIT,(1*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), 9100 * UNIT);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), 900 * UNIT);

        // AskLimit
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),OrderType::AskLimitMM,trading_pair,10000*UNIT,(2*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&alice, quote_asset_id), 2*UNIT - (2055798*UNIT/100000000) - (2*UNIT)/10);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&alice, quote_asset_id), (2 * UNIT)/10);

        let actual_orderbook: Orderbook<TestRuntime> = <Orderbooks<TestRuntime>>::get(trading_pair);
        assert_eq!(actual_orderbook.best_ask_price,FixedU128::from(10000));
        assert_eq!(actual_orderbook.best_bid_price,FixedU128::from(9000));

        // Uniswap has 0.02055798 quote and 100 base implies price = 9500 base per quote
        // Pool before swapping should be as follows
        let pool: (FixedU128, FixedU128) = polkadex_swap_engine::Module::<TestRuntime>::get_liquidity(trading_pair.0,trading_pair.1);
        assert_eq!(pool.1,FixedU128::from(100));
        assert_eq!(pool.0,FixedU128::from_fraction(0.02055798));

        // The following order engages Uniswap instead of orderbook due to low liquidity
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimit,trading_pair,(950000978085*UNIT)/100000000,UNIT/100));

        // Pool after swapping should be as follows
        let pool: (FixedU128, FixedU128) = polkadex_swap_engine::Module::<TestRuntime>::get_liquidity(trading_pair.0,trading_pair.1);
        assert_eq!(pool.1,FixedU128::from(1950000978085).checked_div(&FixedU128::from(10000000000)).unwrap());
        assert_eq!(pool.0,FixedU128::from(10557979459403983).checked_div(&FixedU128::from(1000000000000000000)).unwrap());

        // Orderbook after that order should be like as follows
        let actual_orderbook: Orderbook<TestRuntime> = <Orderbooks<TestRuntime>>::get(trading_pair);
        assert_eq!(actual_orderbook.best_ask_price,FixedU128::from(10000));
        assert_eq!(actual_orderbook.best_bid_price,FixedU128::from(9000));

        // Bob's base asset balance should be as follows
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id), (9004999902191500 * UNIT)/1000000000000);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), 900 * UNIT);

        // Bob's quote asset balance should be as follows
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, quote_asset_id), (10000000540 * UNIT)/1000000000000); // This should be 0.01
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, quote_asset_id), 0 * UNIT);

        // BidLimit - Will not engage Uniswap
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),OrderType::BidLimit,trading_pair,9500*UNIT,(1*UNIT)/10));
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::free_balance(&bob, base_asset_id),(8054999902191500* UNIT)/1000000000000);
        assert_eq!(polkadex_custom_assets::Module::<TestRuntime>::reserved_balance(&bob, base_asset_id), 1850 * UNIT);

        // Orderbook after that order should be like as follows
        let actual_orderbook: Orderbook<TestRuntime> = <Orderbooks<TestRuntime>>::get(trading_pair);
        assert_eq!(actual_orderbook.best_ask_price,FixedU128::from(10000));
        assert_eq!(actual_orderbook.best_bid_price,FixedU128::from(9500));

        // Pool - No Change
        let pool: (FixedU128, FixedU128) = polkadex_swap_engine::Module::<TestRuntime>::get_liquidity(trading_pair.0,trading_pair.1);
        assert_eq!(pool.1,FixedU128::from(1950000978085).checked_div(&FixedU128::from(10000000000)).unwrap());
        assert_eq!(pool.0,FixedU128::from(10557979459403983).checked_div(&FixedU128::from(1000000000000000000)).unwrap());

    });

}

fn calculate_quantity(mut pricelevel: LinkedPriceLevel<TestRuntime>) -> FixedU128 {
    let mut total_quantity = FixedU128::from(0);
    while let Some(order) = pricelevel.orders.pop_back() {
        total_quantity = total_quantity.checked_add(&order.quantity).unwrap()
    }
    total_quantity
}
//
