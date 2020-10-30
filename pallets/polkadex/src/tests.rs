use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::Hash;

use crate::{Error, LinkedPriceLevel, mock::*, mock};
use crate::OrderType::{AskLimit, AskMarket, BidLimit, BidMarket};

use super::*;

const UNIT: u128 = 1_000_000_000_000;

type System = frame_system::Module<Test>;

// Creates two token assets for trading
// Alice - Token #1 - 10000 Units
// Bob - Token #2 - 1 Unit.
fn setup_balances() {
    let alice: u64 = 1;
    let bob: u64 = 2;
    let options_alice = pallet_generic_asset::AssetOptions::<u128, u64> {
        initial_issuance: 10000 * UNIT,
        permissions: Default::default(),
    };
    // Creates first asset to alice's account
    assert_ok!(pallet_generic_asset::Module::<Test>::create_asset(None,Some(alice),options_alice));
    let options_bob = pallet_generic_asset::AssetOptions::<u128, u64> {
        initial_issuance: 1 * UNIT,
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

//Executes some pre-defined trades and checks if the order book state is as expected.
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
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8200*UNIT,(2*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8000*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8500*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8400*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,9000*UNIT,(1*UNIT)/10));
        // Place some random sell limit orders from Bob
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10750*UNIT,(2*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,11000*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10600*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10400*UNIT,(1*UNIT)/10));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10000*UNIT,(1*UNIT)/10));

        // Balances of Token #1 for Alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 4970));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 5030));
        // Balances of Token #2 for Bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), (UNIT / 10) * 4);
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), (UNIT / 10) * 6);

        // Place some random market orders
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,500*UNIT,0));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskMarket,trading_pair,0,(UNIT/100)*5));
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,1646*UNIT,0));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskMarket,trading_pair,0,(UNIT/100)*16));

        // Balances of Token #1 for Alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 2824));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 3196));
        // Balances of Token #2 for Bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), (UNIT / 100) * 19);
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), (UNIT / 100) * 39);

        // Partial limit orders for Alice ( Token1 ) and Bob ( Token 2)
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,10600*UNIT,(5*UNIT)/100));
        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,8400*UNIT,(5*UNIT)/100));

        // TODO: Just for checking orderbook state
        let orderbook: Orderbook<Test> = <Orderbooks<Test>>::get(trading_pair);
        let best_ask_pricelevel: LinkedPriceLevel<Test> = <PriceLevels<Test>>::get(trading_pair, orderbook.best_ask_price);
        assert_eq!(orderbook.best_ask_price,FixedU128::from(10600));
        assert_eq!(calculate_quantity(best_ask_pricelevel.clone()),FixedU128::from_fraction(0.04));
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice),2294*UNIT);
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob),(14*UNIT)/100);

        // Full+half queue limit orders for Alice ( Token1 ) and Bob ( Token 2)
        assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,10750*UNIT,(14*UNIT)/100));
        // TODO: This is where the problem is
        // TODO: Just for checking orderbook state
        let orderbook: Orderbook<Test> = <Orderbooks<Test>>::get(trading_pair);
        let best_ask_pricelevel: LinkedPriceLevel<Test> = <PriceLevels<Test>>::get(trading_pair, orderbook.best_ask_price);
        assert_eq!(orderbook.best_ask_price,FixedU128::from(10750));
        assert_eq!(calculate_quantity(best_ask_pricelevel.clone()),FixedU128::from_fraction(0.1));

        println!("After Alice USD free balance: {}", pallet_generic_asset::Module::<Test>::free_balance(&1, &alice));
        println!("After Bob BTC free balance: {}", pallet_generic_asset::Module::<Test>::free_balance(&2, &bob));

        assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,8200*UNIT,(14*UNIT)/100));

        // Read the block chain state for verifying

        // Balances of Token #1 for Alice
        // If buyer protection enabled, Token #1 free balance for Alice = 795
        // TODO: For some weird reason this is not working.
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 795));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 1620));
        // Balances of Token #2 for Alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), (80 * UNIT) / 100);
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), 0);
        // Balances of Token #1 for Bob
        // If buyer protection enabled, Token #1 free balance for Bob = 7585
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob), 7585 * UNIT);
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob), 0);
        // Balances of Token #2 for Bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), ((UNIT / 1000) * 0));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), ((UNIT / 1000) * 200));
    });
}

// Trying to execute orders with price and quantity values that can underflow or overflow.
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

// Executing order with trading pair that is not yet registered.
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

// The trader that creates a new trading pair doesn't have enough SpendingCurrency to reserve
#[test]
fn correct_error_for_insufficient_balance_to_register_trading_pair() {
    new_test_ext().execute_with(|| {
        let charlie: u64 = 3;
        assert_noop!(DEXModule::register_new_orderbook(Origin::signed(charlie),2,1),Error::<Test>::InsufficientAssetBalance);
    });
}

// Trying to register already existing trading pairs or assets
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

// Trying to cancel orders in the system
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

// // Checks if the market data collection
// #[test]
// fn check_market_data() {
//     new_test_ext().execute_with(|| {
//         let alice: u64 = 1;
//         let bob: u64 = 2;
//         let trading_pair = create_trading_pair_id(&2, &1);
//         // Creates two assets using Alice's and Bob's Accounts.
//         setup_balances();
//         assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
//         // Place some random buy orders from Alice
//         // Place some random buy orders from Alice
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,820*UNIT,(2*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,800*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,850*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,840*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,900*UNIT,(1*UNIT)/10));
//         // Place some random sell limit orders from Bob
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1075*UNIT,(2*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1100*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1060*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1040*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,1000*UNIT,(1*UNIT)/10));
//         // Place some random market orders
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,(UNIT/100)*5,0));
//         let mut market_data: MarketData = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(1000),
//             high: FixedU128::from(1000),
//             volume: FixedU128::from_fraction(0.05),
//             open: FixedU128::from(1000),
//             close: FixedU128::from(1000),
//         });
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskMarket,trading_pair,0,(UNIT/1000)*5));
//         market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(900),
//             high: FixedU128::from(1000),
//             volume: FixedU128::from_fraction(4.55),
//             open: FixedU128::from(1000),
//             close: FixedU128::from(900),
//         });
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidMarket,trading_pair,(UNIT/1000)*16,0));
//         market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(900),
//             high: FixedU128::from(1000),
//             volume: FixedU128::from_fraction(4.566),
//             open: FixedU128::from(1000),
//             close: FixedU128::from(1000),
//         });
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskMarket,trading_pair,0,(UNIT/1000)*16));
//         market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(900),
//             high: FixedU128::from(1000),
//             volume: FixedU128::from_fraction(18.966),
//             open: FixedU128::from(1000),
//             close: FixedU128::from(900),
//         });
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,850*UNIT,(80*UNIT)/1000));
//         market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(850),
//             high: FixedU128::from(1000),
//             volume: FixedU128::from_fraction(90.916),
//             open: FixedU128::from(1000),
//             close: FixedU128::from(850),
//         });
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,1040*UNIT,UNIT/10));
//         market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(850),
//             high: FixedU128::from(1040),
//             volume: FixedU128::from_fraction(190.91864),
//             open: FixedU128::from(1000),
//             close: FixedU128::from(1040),
//         });
//     });
// }
// #[test]
// fn temp_12() {
//     new_test_ext().execute_with(|| {
//         let alice: u64 = 1;
//         let bob: u64 = 2;
//         let trading_pair = create_trading_pair_id(&2, &1);
//         // Creates two assets using Alice's and Bob's Accounts.
//         setup_balances();
//         assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
//
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8200*UNIT,(2*UNIT)/10));
//         let market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(0),
//             high: FixedU128::from(0),
//             volume: FixedU128::from_fraction(0.0),
//             open: FixedU128::from(0),
//             close: FixedU128::from(0),
//         });
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 8360));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 1640));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), (UNIT * 0));
//         // Place bid order
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,5000*UNIT,(1*UNIT)/10));
//
//         let market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(8200),
//             high: FixedU128::from(8200),
//             volume: FixedU128::from_fraction(820.0),
//             open: FixedU128::from(8200),
//             close: FixedU128::from(8200),
//         });
//
//         // Free and reserved balance of Alice
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 8360));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), (UNIT * 1)/10);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 820));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), (UNIT * 0));
//
//         // Free and reserved balance of Bob
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob), (UNIT * 820));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), (UNIT * 9)/10);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), (UNIT * 0));
//
//         // New ask limit order
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,7000*UNIT,(1*UNIT)/10));
//
//         // Chexk Market data
//         let market_data = <MarketInfo<Test>>::get(trading_pair, System::block_number());
//         assert_eq!(market_data, MarketData {
//             low: FixedU128::from(8200),
//             high: FixedU128::from(8200),
//             volume: FixedU128::from_fraction(1640.0),
//             open: FixedU128::from(8200),
//             close: FixedU128::from(8200),
//         });
//
//         // Free and reserved balance of Alice
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 8360));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), (UNIT * 2)/10);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), (UNIT * 0));
//
//         // Free and reserved balance of Bob
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob), (UNIT * 1640));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), (UNIT * 8)/10);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), (UNIT * 0));
//
//
//
//
//
//
//     });

//     new_test_ext().execute_with(|| {
//         let alice: u64 = 1;
//         let bob: u64 = 2;
//         let trading_pair = create_trading_pair_id(&2, &1);
//         // Creates two assets using Alice's and Bob's Accounts.
//         setup_balances();
//         assert_ok!(DEXModule::register_new_orderbook(Origin::signed(alice),2,1));
//
//         // Bid Limit order by alice
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8200*UNIT,(2*UNIT)/10)); //1640  //1640  //8360
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8000*UNIT,(1*UNIT)/10)); //800   //2440  //7560
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8500*UNIT,(1*UNIT)/10)); //850   //3290  //6710
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,8400*UNIT,(1*UNIT)/10)); //840   //4130  //5870
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,9000*UNIT,(1*UNIT)/10)); //900   //5030Res  //4970Rem
//         /// Checkup (Alice)
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 4970));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 5030));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), (UNIT * 0));
//         let bid_level = <BidsLevels<Test>>::get(trading_pair);
//         //assert_eq!(bid_level, Vec::new());
//
//         /// Ask Limit order by bob
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10750*UNIT,(2*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,11000*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10600*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10400*UNIT,(1*UNIT)/10));
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,10000*UNIT,(1*UNIT)/10)); //0.4 Rem //0.6Res
//
//         /// Checkup (Bob)
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), (UNIT * 4)/10);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), (UNIT * 6)/10);
//         let ask_level = <AsksLevels<Test>>::get(trading_pair);
//         //assert_eq!(ask_level, Vec::new());
//
//         assert_ok!(DEXModule::submit_order(Origin::signed(alice),BidLimit,trading_pair,10600*UNIT,(14*UNIT)/100)); //10600 * 0.14 = 1484
//
//         /// Checkup (Alice)
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice), (UNIT * 3486)); //3554
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice), (UNIT * 14)/100);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice), (UNIT * 5060));  //Coming - 5060 - 30? // done (executing 5000 ask limit)
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice), (UNIT * 0));
//
//         assert_ok!(DEXModule::submit_order(Origin::signed(bob),AskLimit,trading_pair,8400*UNIT,(5*UNIT)/100));
//
//         /// Checkup (Bob)
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob), (UNIT * 500));
//         assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob), (UNIT * 4)/10);
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob), (UNIT * 0));
//         assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob), (UNIT * 6)/10);
//
//
//
//
//
//     });
// }

fn create_trading_pair_id(quote_asset_id: &u32, base_asset_id: &u32) -> <mock::Test as frame_system::Trait>::Hash {
    (quote_asset_id, base_asset_id).using_encoded(<Test as frame_system::Trait>::Hashing::hash)
}

fn calculate_quantity(mut pricelevel: LinkedPriceLevel<Test>) -> FixedU128 {
    let mut total_quantity = FixedU128::from(0);
    while let Some(order) = pricelevel.orders.pop_back() {
        total_quantity = total_quantity.checked_add(&order.quantity).unwrap()
    }
    total_quantity
}

