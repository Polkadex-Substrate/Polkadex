use crate::{mock::*, mock};
use frame_support::assert_ok;
use frame_system::{ensure_signed};
use crate::OrderType::{BidLimit, AskLimit, BidMarket, AskMarket};
use codec::Encode;
use sp_runtime::traits::Hash;


const UNIT: u128 = 1_000_000_000_000;

fn setup_balances(){
    let alice: u64 = 1;
    let bob: u64= 2;
    let options_alice = pallet_generic_asset::AssetOptions::<u128,u64>{
        initial_issuance: 1000*UNIT,
        permissions: Default::default() };
    // Creates first asset to alice's account
    assert_ok!(pallet_generic_asset::Module::<Test>::create_asset(None,Some(alice),options_alice));
    let options_bob = pallet_generic_asset::AssetOptions::<u128,u64>{
        initial_issuance: UNIT,
        permissions: Default::default() };
    // Creates second asset to bob's account
    assert_ok!(pallet_generic_asset::Module::<Test>::create_asset(None,Some(bob),options_bob));
}

#[test]
fn setup_balances_test(){
    new_test_ext().execute_with(|| {
        setup_balances();
    });
}

#[test]
fn check_trading_engine(){
    new_test_ext().execute_with(|| {

        let alice: u64 = 1;
        let bob: u64 = 2;
        let trading_pair = create_trading_pair_id(&2,&1);
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
        // Balances of Token #1 for alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &alice),((UNIT/1000)*496934));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &alice),((UNIT/10)*4841));
        // Balances of Token #2 for alice
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &alice),((UNIT/1000000)*21066));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &alice),0);
        // Balances of Token #1 for bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&1, &bob),((UNIT/1000)*18966));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&1, &bob),0);
        // Balances of Token #2 for bob
        assert_eq!(pallet_generic_asset::Module::<Test>::free_balance(&2, &bob),((UNIT/1000)*379));
        assert_eq!(pallet_generic_asset::Module::<Test>::reserved_balance(&2, &bob),((UNIT/1000000)*599934));

    });
}

fn create_trading_pair_id(quote_asset_id: &u32, base_asset_id: &u32) -> <mock::Test as frame_system::Trait>::Hash {

    (quote_asset_id, base_asset_id).using_encoded(<Test as frame_system::Trait>::Hashing::hash)
}

// #[test]
// fn it_works_for_default_value() {
// 	new_test_ext().execute_with(|| {
// 		// Dispatch a signed extrinsic.
// 		assert_ok!(TemplateModule::do_something(Origin::signed(1), 42));
// 		// Read pallet storage and assert an expected result.
// 		assert_eq!(TemplateModule::something(), Some(42));
// 	});
// }
//
// #[test]
// fn correct_error_for_none_value() {
// 	new_test_ext().execute_with(|| {
// 		// Ensure the expected error is thrown when no value is present.
// 		assert_noop!(
// 			TemplateModule::cause_error(Origin::signed(1)),
// 			Error::<Test>::NoneValue
// 		);
// 	});
// }
