use codec::Encode;
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::Hash;
use super::*;
use crate::{ Error, mock::*};


const UNIT: u128 = 1_000_000_000_000;



fn init_storage_asset() {
    let alice: u64 = 1;
    assert_ok!(polkadex_custom_assets::Module::<Test>::create_token(Origin::signed(alice), 10*UNIT, 0));
    let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 10*UNIT);

    assert_ok!(polkadex_custom_assets::Module::<Test>::create_token(Origin::signed(alice),10*UNIT, 0));
    let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 10*UNIT);

    assert_ok!(polkadex_custom_assets::Module::<Test>::create_token(Origin::signed(alice),10*UNIT, 0));
    let second_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 10*UNIT);

    assert_ok!(polkadex_custom_assets::Module::<Test>::create_token(Origin::signed(alice),10*UNIT, 0));
    let second_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 10*UNIT);

    assert_ok!(polkadex_custom_assets::Module::<Test>::create_token(Origin::signed(alice),10*UNIT, 0));
    let second_asset_id = (4 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
    assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 10*UNIT);

}

// fn get_amount(supply_pool: FixedU128, target_pool: FixedU128, supply_amount: FixedU128) -> FixedU128 {
//
//         let swap_fee: FixedU128 = FixedU128::from(3).checked_div(&FixedU128::from(1000)).unwrap();
//
//         let fee_reduced_supply_amount: FixedU128 = supply_amount.saturating_mul(swap_fee);
//
//         let numerator: FixedU128 = target_pool.saturating_mul(supply_pool);
//
//
//         let denominator: FixedU128 = supply_pool.saturating_add(supply_amount);
//         let denominator: FixedU128 = denominator.saturating_sub(fee_reduced_supply_amount);
//
//
//         let target_amount: FixedU128 = numerator.checked_div(&denominator)
//             .unwrap_or_else(FixedU128::zero);
//
//         target_amount
//
// }

fn f128 (x:u128) -> FixedU128 {
    PolkadexSwapEngine::convert_balance_to_fixedU128(x).unwrap()
}

#[test]
pub fn test_register_swap_pair () {


    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, UNIT, 2*UNIT));
        let liquidity_pool: (FixedU128, FixedU128, FixedU128)  = <LiquidityPool<Test>>::get((second_asset_id, first_asset_id));
        let liquidity_pool_holdings: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (second_asset_id, first_asset_id)));
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 9*UNIT);
        assert_eq!(liquidity_pool.1, PolkadexSwapEngine::convert_balance_to_fixedU128(UNIT).unwrap());
        assert_eq!(liquidity_pool.0, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT).unwrap());
        assert_eq!(liquidity_pool.2, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT).unwrap());
        assert_eq!(liquidity_pool_holdings, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT).unwrap());

    });

    //Test Errors
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);

        //InsufficientLiquidity
        assert_noop!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 0u128, 200u128), <Error<Test>>::InsufficientLiquidity);
        assert_noop!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 100u128, 0u128), <Error<Test>>::InsufficientLiquidity);
        assert_noop!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 0u128, 0u128), <Error<Test>>::InsufficientLiquidity);

        //TradingPairNotAllowed
        assert_ok!(PolkadexSwapEngine::register_swap_pair(Origin::signed(alice), first_asset_id, second_asset_id, 100u128, 200u128));
        assert_noop!(PolkadexSwapEngine::register_swap_pair(Origin::signed(alice), first_asset_id, second_asset_id, 100u128, 200u128), <Error<Test>>::TradingPairNotAllowed);

    });

}

#[test]
pub fn test_swap_with_exact_supply () {


    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let third_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);

        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, second_asset_id, third_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, third_asset_id, fourth_asset_id, 2*UNIT, 2*UNIT));

        let path = vec![second_asset_id, third_asset_id, fourth_asset_id];
        assert_ok!(PolkadexSwapEngine::swap_with_exact_supply(Origin::signed(alice), path, UNIT, UNIT/10));
        let liquidity_pool_23: (FixedU128, FixedU128, FixedU128)  = <LiquidityPool<Test>>::get((second_asset_id, third_asset_id));
        let liquidity_pool_holdings_23: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (second_asset_id, third_asset_id)));
        let liquidity_pool_34: (FixedU128, FixedU128, FixedU128)  = <LiquidityPool<Test>>::get((fourth_asset_id, third_asset_id));
        let liquidity_pool_holdings_34: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (fourth_asset_id, third_asset_id)));
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 5*UNIT);

        //assert_eq!(liquidity_pool_23.1, get_amount(f128(2*UNIT), f128(2*UNIT), f128(UNIT))); // This should work
        assert_eq!(liquidity_pool_23.0, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT+UNIT).unwrap());
        assert_eq!(liquidity_pool_23.2, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT).unwrap());
        assert_eq!(liquidity_pool_holdings_23, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT).unwrap());
        assert_eq!(liquidity_pool_holdings_34, PolkadexSwapEngine::convert_balance_to_fixedU128(2*UNIT).unwrap());
        assert_eq!(liquidity_pool_34.1, f128(2*UNIT)+f128(2*UNIT)-liquidity_pool_23.1);
        //assert_eq!(liquidity_pool_34.0, get_amount(f128(2*UNIT), f128(2*UNIT), f128(2*UNIT)-liquidity_pool_23.1)); This should work

    });


    // Error Test
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let third_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let path = vec![second_asset_id, third_asset_id, fourth_asset_id];
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, second_asset_id, third_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, third_asset_id, fourth_asset_id, 2*UNIT, 2*UNIT));
        assert_noop!(PolkadexSwapEngine::swap_with_exact_supply(Origin::signed(alice), path, UNIT, UNIT), <Error<Test>>::InsufficientTargetAmount);

        // Path length less then 2
        let path = vec![second_asset_id];
        assert_noop!(PolkadexSwapEngine::swap_with_exact_supply(Origin::signed(alice), path, 8*UNIT, UNIT/5), <Error<Test>>::InvalidTradingPathLength);

        // TradingPairNotAllowed
        let fifth_asset_id = (4 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, fifth_asset_id, 2*UNIT, 2*UNIT));
        let path = vec![first_asset_id,second_asset_id, third_asset_id, fourth_asset_id, fifth_asset_id];
        assert_noop!(PolkadexSwapEngine::do_swap_with_exact_supply(&alice, &path, 8*UNIT, UNIT/5, None), <Error<Test>>::TradingPairNotAllowed);

    });
    // Error Test
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let third_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);

        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, second_asset_id, third_asset_id, UNIT/9, UNIT/9));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, third_asset_id, fourth_asset_id, 2*UNIT, 2*UNIT));

        let path = vec![second_asset_id, third_asset_id, fourth_asset_id];
        assert_noop!(PolkadexSwapEngine::swap_with_exact_supply(Origin::signed(alice), path, UNIT, UNIT/10), <Error<Test>>::InsufficientTargetAmount);
    });

}

#[test]
pub fn test_swap_with_exact_target () {
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10 * UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10 * UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let third_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10 * UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10 * UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);

        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, second_asset_id, third_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, third_asset_id, fourth_asset_id, 2*UNIT, 2*UNIT));

        let path = vec![second_asset_id, third_asset_id, fourth_asset_id];
        assert_ok!(PolkadexSwapEngine::swap_with_exact_target(Origin::signed(alice), path, UNIT/10, UNIT*5));
        let liquidity_pool_23: (FixedU128, FixedU128, FixedU128) = <LiquidityPool<Test>>::get((second_asset_id, third_asset_id));
        let liquidity_pool_holdings_23: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (second_asset_id, third_asset_id)));
        let liquidity_pool_34: (FixedU128, FixedU128, FixedU128) = <LiquidityPool<Test>>::get((fourth_asset_id, third_asset_id));
        let liquidity_pool_holdings_34: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (fourth_asset_id, third_asset_id)));
        assert_eq!(liquidity_pool_holdings_23, f128(2*UNIT));
        assert_eq!(liquidity_pool_holdings_34, f128(2*UNIT));
        assert_eq!(liquidity_pool_34.0, f128(2*UNIT) - f128(UNIT/10));
        assert_eq!(liquidity_pool_23.1, f128(4*UNIT) - liquidity_pool_34.1);
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, fourth_asset_id), 8*UNIT+UNIT/10);

    });

    // Error Test
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let third_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let path = vec![second_asset_id, third_asset_id, fourth_asset_id];
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, second_asset_id, third_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, third_asset_id, fourth_asset_id, 2*UNIT, 2*UNIT));
        assert_noop!(PolkadexSwapEngine::swap_with_exact_target(Origin::signed(alice), path, UNIT, UNIT), <Error<Test>>::ZeroSupplyAmount);

        // Path length less then 2
        let path = vec![second_asset_id];
        assert_noop!(PolkadexSwapEngine::swap_with_exact_target(Origin::signed(alice), path, 8*UNIT, UNIT/5), <Error<Test>>::InvalidTradingPathLength);

        // TradingPairNotAllowed
        let fifth_asset_id = (4 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, fifth_asset_id, 2*UNIT, 2*UNIT));
        let path = vec![first_asset_id,second_asset_id, third_asset_id, fourth_asset_id, fifth_asset_id];
        assert_noop!(PolkadexSwapEngine::swap_with_exact_target(Origin::signed(alice), path, 8*UNIT, UNIT/5), <Error<Test>>::TradingPairNotAllowed);

    });

    // Error Test
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let third_asset_id = (2 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (3 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);

        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, second_asset_id, third_asset_id, UNIT/9, UNIT/9));
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, third_asset_id, fourth_asset_id, 2*UNIT, 2*UNIT));

        let path = vec![second_asset_id, third_asset_id, fourth_asset_id];
        assert_noop!(PolkadexSwapEngine::swap_with_exact_target(Origin::signed(alice), path, UNIT, UNIT), <Error<Test>>::ZeroSupplyAmount);
    });

}

#[test]
pub fn test_add_liqudity () {

    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);

        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        let liquidity_pool_holdings_12_before: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (second_asset_id, first_asset_id)));
        assert_ok!(PolkadexSwapEngine::add_liquidity(Origin::signed(alice), second_asset_id, first_asset_id, UNIT, 3*UNIT));
        let liquidity_pool_12: (FixedU128, FixedU128, FixedU128)  = <LiquidityPool<Test>>::get((second_asset_id, first_asset_id));
        let liquidity_pool_holdings_12_after: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (second_asset_id, first_asset_id)));
        assert_eq!(liquidity_pool_12.0, f128(3*UNIT));
        assert_eq!(liquidity_pool_12.1, f128(3*UNIT));
        assert_eq!(liquidity_pool_holdings_12_before, f128(2*UNIT));
        assert_eq!(liquidity_pool_holdings_12_after, f128(3*UNIT));
    });

    // Test Errors
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_noop!(PolkadexSwapEngine::add_liquidity(Origin::signed(alice), second_asset_id, first_asset_id, UNIT, 0), <Error<Test>>::ProvidedAmountIsZero);
        assert_noop!(PolkadexSwapEngine::add_liquidity(Origin::signed(alice), second_asset_id, first_asset_id, 0, 0), <Error<Test>>::ProvidedAmountIsZero);
        assert_noop!(PolkadexSwapEngine::add_liquidity(Origin::signed(alice), second_asset_id, first_asset_id, 0, UNIT), <Error<Test>>::ProvidedAmountIsZero);

    });

}

#[test]
pub fn test_remove_liqudity () {

    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        let liquidity_pool_holdings_12: FixedU128 = <LiquidityPoolHoldings<Test>>::get((alice, (second_asset_id, first_asset_id)));
        // Check free balance
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 8*UNIT);
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 8*UNIT);
        assert_eq!(liquidity_pool_holdings_12, f128(2*UNIT));
        assert_ok!(PolkadexSwapEngine::remove_liquidity(Origin::signed(alice), first_asset_id, second_asset_id, 1*UNIT));
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 9*UNIT);
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 9*UNIT);
        assert_eq!(liquidity_pool_holdings_12, f128(2*UNIT));

    });

    // Error Test
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        assert_noop!(PolkadexSwapEngine::remove_liquidity(Origin::signed(alice), first_asset_id, second_asset_id, 3*UNIT), <Error<Test>>::LowShare);
        // Wrong Party
        assert_noop!(PolkadexSwapEngine::remove_liquidity(Origin::signed(bob), first_asset_id, second_asset_id, 1*UNIT), <Error<Test>>::LowShare);
    });
}

#[test]
pub fn test_orderbook_functions () {
    new_test_ext().execute_with(|| {
        init_storage_asset();
        let alice: u64 = 1;
        let _bob: u64 = 2;
        let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
        // Check for best deal
        // Argument Provided :- max-supply:- 1*UNIT and min-target 1*UNIT -> It should return none
        assert!(PolkadexSwapEngine::give_best_deal(PolkadexSwapEngine::convert_balance_to_fixedU128(1*UNIT).unwrap(), PolkadexSwapEngine::convert_balance_to_fixedU128(1*UNIT).unwrap(), (first_asset_id, second_asset_id)).is_none());
        // Check for best case
        assert!(PolkadexSwapEngine::give_best_deal(PolkadexSwapEngine::convert_balance_to_fixedU128(1*UNIT).unwrap(), PolkadexSwapEngine::convert_balance_to_fixedU128(UNIT/10).unwrap(), (first_asset_id, second_asset_id)).is_some());

        // Check for best deal and Order execution
        // Argument Provided :- max-supply:- 1*UNIT and min-target 1*UNIT -> It should return false
        assert!(!PolkadexSwapEngine::swap_by_orderbook(&alice, (second_asset_id, first_asset_id), true, PolkadexSwapEngine::convert_balance_to_fixedU128(1 * UNIT).unwrap(), PolkadexSwapEngine::convert_balance_to_fixedU128(UNIT).unwrap(), second_asset_id));
        // Check for best case
        // Check for free balance
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 8*UNIT);
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 8*UNIT);
        let best_deal = PolkadexSwapEngine::give_best_deal(PolkadexSwapEngine::convert_balance_to_fixedU128(1*UNIT).unwrap(), PolkadexSwapEngine::convert_balance_to_fixedU128(UNIT/10).unwrap(), (first_asset_id, second_asset_id)).unwrap();
        assert_eq!(PolkadexSwapEngine::execute_deal(&alice, best_deal, (first_asset_id, second_asset_id)), true);
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 7*UNIT);
        assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, second_asset_id), 8665331998665u128);


    });
    // // Using SwapByOrderBook
    // new_test_ext().execute_with(|| {
    //     init_storage_asset();
    //     let alice: u64 = 1;
    //     let bob: u64 = 2;
    //     let first_asset_id = (0 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Trait>::Hashing::hash);
    //     let second_asset_id = (1 as u64, alice.clone(), PolkadexSwapEngine::convert_balance_to_fixedU128(10*UNIT).unwrap()).using_encoded(<Test as frame_system::Trait>::Hashing::hash);
    //     assert_ok!(PolkadexSwapEngine::do_register_swap_pair(&alice, first_asset_id, second_asset_id, 2*UNIT, 2*UNIT));
    //     assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 8*UNIT);
    //     assert_eq!(polkadex_custom_assets::Module::<Test>::free_balance(&alice, first_asset_id), 8*UNIT);
    //     debug_assert_eq!(PolkadexSwapEngine::swap_by_orderbook(&alice, (first_asset_id, second_asset_id), true, f128(UNIT/10), f128(UNIT/20)), true);
    //
    // });
}
