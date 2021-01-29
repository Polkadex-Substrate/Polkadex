use crate::mock::*;
use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::FixedU128;
use codec::Encode;
use sp_runtime::traits::Hash;
use super::*;
use sp_arithmetic::traits::Bounded;
use sp_core::H256;

use super::*;

const UNIT: u128 = 1_000_000_000_000;

fn setup_native() {
    let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
    let system: u64 = 0;

    let asset_info: AssetInfo<Test> =  AssetInfo{
        total_issuance: FixedU128::from(106),
        issuer: system,
        permissions: Permissions::SystemLevel,
        existential_deposit: FixedU128::from(1)
    };
    let alice: u64 = 1;
    let account_data: AccountData = AccountData{
        free_balance: FixedU128::from(100),
        reserved_balance: FixedU128::from(4),
        misc_frozen: FixedU128::from(1),
        fee_frozen: FixedU128::from(1)
    };
    <Balance<Test>>::insert(&native_asset, &alice, &account_data);
    <NativeAssetId<Test>>::put(native_asset);

    <Assets<Test>>::insert(&native_asset, &asset_info);

}

fn setup_asset_pallet() {
    let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
    let user: u64 = 0;

    let asset_info: AssetInfo<Test> =  AssetInfo{
        total_issuance: FixedU128::from(106),
        issuer: user,
        permissions: Permissions::SystemLevel,
        existential_deposit: FixedU128::from(1)
    };
    let alice: u64 = 1;
    let account_data: AccountData = AccountData{
        free_balance: FixedU128::from(100),
        reserved_balance: FixedU128::from(4),
        misc_frozen: FixedU128::from(1),
        fee_frozen: FixedU128::from(1)
    };
    <Balance<Test>>::insert(&sample_asset, &alice, &account_data);
    <NativeAssetId<Test>>::put(sample_asset);

    <Assets<Test>>::insert(&sample_asset, &asset_info);
}

#[test]
fn test_currency_trait_implementation () {
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        // Test total balance
        assert_eq!(Native::total_balance(&alice), 104u128*UNIT);
        // Test total balance - Wrong account id
        let wrong_id: u64 = 2;
        assert_eq!(Native::total_balance(&wrong_id), 0u128*UNIT);
        // Test Balance Slash - true
        assert_eq!(Native::can_slash(&alice, 60u128*UNIT), true);
        // Test Balance Slash - false
        assert_eq!(Native::can_slash(&alice, 200u128*UNIT), false);
        // Total Issurance
        assert_eq!(Native::total_issuance(), 106u128*UNIT);
        // Minimum Balance
        assert_eq!(Native::minimum_balance(), 1u128);
        // Free Balance
        assert_eq!(Native::free_balance(&alice), 100u128*UNIT);
    });

    // Burn Method
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::burn(50u128*UNIT), PositiveImbalance::new(50u128*UNIT));
    });

    // Burn amount more then total issuance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_eq!(Native::burn(250u128*UNIT), PositiveImbalance::new(106u128*UNIT)); //@Gautham check this
        let assert_info: AssetInfo<Test> = <Assets<Test>>::get(&native_asset);
        assert_eq!(assert_info.total_issuance, FixedU128::from(0));
    });

    // Issue amount
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::issue(250u128*UNIT), NegativeImbalance::new(250u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let assert_info: AssetInfo<Test> = <Assets<Test>>::get(&native_asset);
        assert_eq!(assert_info.total_issuance, FixedU128::from(356));
    });

    // Test Slash - Slashing amount less then Free Balance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::slash(&alice, 50u128*UNIT), (NegativeImbalance::new(50u128*UNIT), 0u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.free_balance, FixedU128::from(50));
        assert_eq!(balance_info.reserved_balance, FixedU128::from(4));
    });

    // Test Slash - Slashing amount less then sum of Reserved and Free Balance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::slash(&alice, 102u128*UNIT), (NegativeImbalance::new(102u128*UNIT), 0u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.free_balance, FixedU128::from(0));
        assert_eq!(balance_info.reserved_balance, FixedU128::from(2));
    });

    // Test Slash - Slashing Equal amount to sum of free and reserved balance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::slash(&alice, 104u128*UNIT), (NegativeImbalance::new(104u128*UNIT), 0u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.free_balance, FixedU128::from(0));
        assert_eq!(balance_info.reserved_balance, FixedU128::from(0));
    });

    // Test Deposit Creating
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::deposit_creating(&alice, 100u128*UNIT), PositiveImbalance::new(100u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.free_balance, FixedU128::from(200));
        let bob: u64 = 2;
        assert_eq!(Native::deposit_creating(&bob, 100u128*UNIT), PositiveImbalance::new(100u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &bob);
        assert_eq!(balance_info.free_balance, FixedU128::from(100));
    });

    // Test Deposit into exiting
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::deposit_into_existing(&alice, 100u128*UNIT), Ok(PositiveImbalance::new(100u128*UNIT)));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.free_balance, FixedU128::from(200));
        let wrong_id: u64 = 2;
        assert_noop!(Native::deposit_into_existing(&wrong_id, 100u128*UNIT), Error::<Test>::AccountNotFound);
    });

    // Test Withdraw
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::withdraw(&alice, 5u128 * UNIT, WithdrawReasons::TRANSACTION_PAYMENT, ExistenceRequirement::AllowDeath), Ok(NegativeImbalance::new(5u128*UNIT)));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.free_balance, FixedU128::from(95));

        // Test Error
        assert_noop!(Native::withdraw(&alice, 500u128 * UNIT, WithdrawReasons::TRANSACTION_PAYMENT, ExistenceRequirement::AllowDeath), Error::<Test>::InsufficientBalance);
    });

}

#[test]
fn test_reserve_trait_implementation () {
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        // Value greater then free balance
        assert_eq!(Native::can_reserve(&alice, 200u128*UNIT), false);
        // // Value lower thrn free balance
        assert_eq!(Native::can_reserve(&alice, 50u128*UNIT), true);
        // Give asset balance
        assert_eq!(Native::reserved_balance(&alice), 4u128*UNIT);
        // Reserve less then free balance
        assert_ok!(Native::reserve(&alice, 50u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.reserved_balance, FixedU128::from(54));
        // Reserve more then free balance
        assert_noop!(Native::reserve(&alice, 500u128*UNIT), Error::<Test>::SubUnderflowOrOverflow);
        // Unreserve more then free balance

        assert_eq!(Native::unreserve(&alice, 500u128*UNIT), (500u128 - 54u128)*UNIT);
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.reserved_balance, FixedU128::from(0));

        // Unreserve zero amount
        assert_eq!(Native::unreserve(&alice, 0u128*UNIT), 0u128*UNIT);
    });

    // Check repatriate_reserved - Slashing more then reserved balance - FreeBalance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let account_data: AccountData = AccountData{
            free_balance: FixedU128::from(1000),
            reserved_balance: FixedU128::from(44),
            misc_frozen: FixedU128::from(1),
            fee_frozen: FixedU128::from(1)
        };
        <Balance<Test>>::insert(&native_asset, &bob, &account_data);
        assert_eq!(Native::repatriate_reserved(&alice, &bob, 20u128*UNIT, BalanceStatus::Free), Ok(16u128*UNIT));
        let bob_balance: AccountData = <Balance<Test>>::get(&native_asset, &bob);
        assert_eq!(bob_balance.free_balance, FixedU128::from(1004));
    });

    // Check repatriate_reserved - Slashing more then reserved balance - ReserveBalance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let account_data: AccountData = AccountData{
            free_balance: FixedU128::from(1000),
            reserved_balance: FixedU128::from(44),
            misc_frozen: FixedU128::from(1),
            fee_frozen: FixedU128::from(1)
        };
        <Balance<Test>>::insert(&native_asset, &bob, &account_data);
        assert_eq!(Native::repatriate_reserved(&alice, &bob, 20u128*UNIT, BalanceStatus::Reserved), Ok(16u128*UNIT));
        let bob_balance: AccountData = <Balance<Test>>::get(&native_asset, &bob);
        assert_eq!(bob_balance.reserved_balance, FixedU128::from(48));
    });

    // Check repatriate_reserved - Slashing more then reserved balance - ReserveBalance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        let bob: u64 = 2;
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let account_data: AccountData = AccountData{
            free_balance: FixedU128::from(1000),
            reserved_balance: FixedU128::from(44),
            misc_frozen: FixedU128::from(1),
            fee_frozen: FixedU128::from(1)
        };
        <Balance<Test>>::insert(&native_asset, &bob, &account_data);
        assert_eq!(Native::repatriate_reserved(&alice, &bob, 2u128*UNIT, BalanceStatus::Reserved), Ok(0u128));
        let bob_balance: AccountData = <Balance<Test>>::get(&native_asset, &bob);
        assert_eq!(bob_balance.reserved_balance, FixedU128::from(46));
    });

    // Test slashed Reserve - Slashing amount less then reserved balance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::slash_reserved(&alice, 1u128*UNIT), (NegativeImbalance::new(1u128*UNIT), 0u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.reserved_balance, FixedU128::from(3));
    });

    // Test slashed Reserve - Slashing amount more then reserved balance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::slash_reserved(&alice, 8u128*UNIT), (NegativeImbalance::new(4u128*UNIT), 4u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.reserved_balance, FixedU128::from(0));
    });

    // Test slashed Reserve - Slashing equal amount to reserved balance
    new_test_ext().execute_with(|| {
        setup_native();
        let alice: u64 = 1;
        assert_eq!(Native::slash_reserved(&alice, 4u128*UNIT), (NegativeImbalance::new(4u128*UNIT), 0u128*UNIT));
        let native_asset =  ("Native").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let balance_info: AccountData = <Balance<Test>>::get(&native_asset, &alice);
        assert_eq!(balance_info.reserved_balance, FixedU128::from(0));
    });
}

#[test]
fn test_module_methods_implementation () {
    // Check for reserve
    new_test_ext().execute_with(|| {
        setup_asset_pallet();
        let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let alice: u64 = 1;
        assert_ok!(PolkadexCustomAssetsModule::reserve(&alice, sample_asset, 80u128*UNIT));
        let alice_balance: AccountData = <Balance<Test>>::get(&sample_asset, &alice);
        assert_eq!(alice_balance.free_balance, FixedU128::from(20));
        assert_eq!(alice_balance.reserved_balance, FixedU128::from(84));
    });
    // Check for underflow - reserve
    new_test_ext().execute_with(|| {
        setup_asset_pallet();
        let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let alice: u64 = 1;
        assert_noop!(PolkadexCustomAssetsModule::reserve(&alice, sample_asset, 120u128*UNIT), Error::<Test>::SubUnderflowOrOverflow);
        // Wrong Account Id
        let wrong_id: u64 =2;
        assert_noop!(PolkadexCustomAssetsModule::reserve(&wrong_id, sample_asset, 500u128*UNIT), Error::<Test>::AccountNotFound);
        // Wrong Asset Id
        let wrong_asset =  ("Wrong").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_noop!(PolkadexCustomAssetsModule::reserve(&alice, wrong_asset, 500u128*UNIT), Error::<Test>::AccountNotFound);

    });

    // Check for unreserve
    new_test_ext().execute_with(|| {
        setup_asset_pallet();
        let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let alice: u64 = 1;
        assert_ok!(PolkadexCustomAssetsModule::unreserve(&alice, sample_asset, 2u128*UNIT));
        let alice_balance: AccountData = <Balance<Test>>::get(&sample_asset, &alice);
        assert_eq!(alice_balance.free_balance, FixedU128::from(102));
        assert_eq!(alice_balance.reserved_balance, FixedU128::from(2));
        // Wrong Account Id
        let wrong_id: u64 =2;
        assert_noop!(PolkadexCustomAssetsModule::unreserve(&wrong_id, sample_asset, 500u128*UNIT), Error::<Test>::AccountNotFound);
        // Wrong Asset Id
        let wrong_asset =  ("Wrong").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        assert_noop!(PolkadexCustomAssetsModule::unreserve(&alice, wrong_asset, 500u128*UNIT), Error::<Test>::AccountNotFound);

    });

    // Check for underflow - unreserve
    new_test_ext().execute_with(|| {
        setup_asset_pallet();
        let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let alice: u64 = 1;
        assert_noop!(PolkadexCustomAssetsModule::unreserve(&alice, sample_asset, 120u128*UNIT), Error::<Test>::SubUnderflowOrOverflow);
    });

    // Check for free balance
    new_test_ext().execute_with(|| {
        setup_asset_pallet();
        let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let alice: u64 = 1;
        assert_eq!(PolkadexCustomAssetsModule::free_balance(&alice, sample_asset), 100u128*UNIT);
        // Wrong account id
        let wrong_id: u64 =2;
        assert_eq!(PolkadexCustomAssetsModule::free_balance(&wrong_id, sample_asset), 0u128*UNIT);
    });

    // Check for Reserved Balance
    new_test_ext().execute_with(|| {
        setup_asset_pallet();
        let sample_asset =  ("Sample").using_encoded(<Test as frame_system::Config>::Hashing::hash);
        let alice: u64 = 1;
        assert_eq!(PolkadexCustomAssetsModule::reserved_balance(&alice, sample_asset), 4u128*UNIT);
        // Wrong account id
        let wrong_id: u64 =2;
        assert_eq!(PolkadexCustomAssetsModule::reserved_balance(&wrong_id, sample_asset), 0u128*UNIT);

    });


}

#[test]
fn test_genesis_storage(){
    new_test_ext().execute_with(||{
        let native_asset = <NativeAssetId<Test>>::get();
        let asset_info: AssetInfo<Test> = <Assets<Test>>::get(native_asset);
        assert_eq!(asset_info.total_issuance,FixedU128::from(3000_000_000_000));
        let test_acc1: AccountData = <Balance<Test>>::get(native_asset,0);
        let test_acc2: AccountData = <Balance<Test>>::get(native_asset,1);
        let test_acc3: AccountData = <Balance<Test>>::get(native_asset,2);
        assert_eq!(test_acc1.free_balance,FixedU128::from(1000_000_000_000));
        assert_eq!(test_acc2.free_balance,FixedU128::from(1000_000_000_000));
        assert_eq!(test_acc3.free_balance,FixedU128::from(1000_000_000_000));
    })
}


