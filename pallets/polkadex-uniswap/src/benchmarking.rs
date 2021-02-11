#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_system::{EventRecord, RawOrigin};
use frame_support::ensure;
use frame_benchmarking::{benchmarks, account, whitelisted_caller};
use polkadex_custom_assets::{Balance, AccountData};

use sp_core::H256;
use sp_std::prelude::*;
use codec::Encode;
use crate::Module as PolkadexSwap;
const SEED: u32 = 0;
const UNIT: u32 = 1_000_000;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Module::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

fn setup_balance<T: Config>() {
    let caller: T::AccountId = account("caller", 0, SEED);
    let first_asset_id = (1 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
    let second_asset_id = (2 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
    let third_asset_id = (3 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
    let fourth_asset_id = (4 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
    let fifth_asset_id = (5 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
    let account_data = polkadex_custom_assets::AccountData {
        free_balance: PolkadexSwap::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
        reserved_balance: FixedU128::from(0),
        misc_frozen: FixedU128::from(0),
        fee_frozen: FixedU128::from(0),
    };
    <Balance<T>>::insert(&first_asset_id.clone(), &caller.clone(), &account_data);
    <Balance<T>>::insert(&second_asset_id.clone(), &caller.clone(), &account_data);
    <Balance<T>>::insert(&third_asset_id.clone(), &caller.clone(), &account_data);
    <Balance<T>>::insert(&fourth_asset_id.clone(), &caller.clone(), &account_data);
    <Balance<T>>::insert(&fifth_asset_id.clone(), &caller.clone(), &account_data);
    PolkadexSwap::<T>::register_swap_pair(RawOrigin::Signed(caller.clone()).into(), first_asset_id, second_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT));
    PolkadexSwap::<T>::register_swap_pair(RawOrigin::Signed(caller.clone()).into(), second_asset_id, third_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT));
    PolkadexSwap::<T>::register_swap_pair(RawOrigin::Signed(caller.clone()).into(), third_asset_id, fourth_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT));
    PolkadexSwap::<T>::register_swap_pair(RawOrigin::Signed(caller.clone()).into(), fourth_asset_id, fifth_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT));

}


benchmarks! {
    register_swap_pair {

        let caller: T::AccountId = account("caller", 0, SEED);
        let first_asset_id = (1 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let second_asset_id = (2 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let account_data = polkadex_custom_assets::AccountData {
	        free_balance: PolkadexSwap::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	    <Balance<T>>::insert(&first_asset_id.clone(), &caller.clone(), &account_data);
	    <Balance<T>>::insert(&second_asset_id.clone(), &caller.clone(), &account_data);


    }: _(RawOrigin::Signed(caller), first_asset_id, second_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT))

    verify {
		assert_last_event::<T>(Event::<T>::RegisteredNewSwapPair(first_asset_id, second_asset_id).into());
	}

	swap_with_exact_supply {
	    let caller: T::AccountId = account("caller", 0, SEED);
	    setup_balance::<T>();
	    let first_asset_id = (1 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let second_asset_id = (2 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let third_asset_id = (3 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (4 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let fifth_asset_id = (5 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let path = vec![first_asset_id, second_asset_id, third_asset_id, fourth_asset_id, fifth_asset_id];
	    let account_data = polkadex_custom_assets::AccountData {
	        free_balance: PolkadexSwap::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	    <Balance<T>>::insert(&first_asset_id.clone(), &caller.clone(), &account_data);
	    <Balance<T>>::insert(&fifth_asset_id.clone(), &caller.clone(), &account_data);

	}: _(RawOrigin::Signed(caller.clone()), path.clone(), T::Balance::from(UNIT), T::Balance::from(UNIT/100))

	verify {
        assert_last_event::<T>(Event::<T>::SwapedWithExactSupply(caller, path).into());
	}

	swap_with_exact_target{
	    let caller: T::AccountId = account("caller", 0, SEED);
	    setup_balance::<T>();
	    let first_asset_id = (1 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let second_asset_id = (2 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let third_asset_id = (3 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let fourth_asset_id = (4 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let fifth_asset_id = (5 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let path = vec![first_asset_id, second_asset_id, third_asset_id, fourth_asset_id, fifth_asset_id];
	    let account_data = polkadex_custom_assets::AccountData {
	        free_balance: PolkadexSwap::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	    <Balance<T>>::insert(&first_asset_id.clone(), &caller.clone(), &account_data);
	}: _(RawOrigin::Signed(caller.clone()), path.clone(), T::Balance::from(UNIT/10), T::Balance::from(UNIT))

    verify {
        assert_last_event::<T>(Event::<T>::SwapedWithExactTarget(caller, path).into());
	}

	add_liquidity{
	    let caller: T::AccountId = account("caller", 0, SEED);
        let first_asset_id = (1 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let second_asset_id = (2 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let account_data = polkadex_custom_assets::AccountData {
	        free_balance: PolkadexSwap::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	    <Balance<T>>::insert(&first_asset_id.clone(), &caller.clone(), &account_data);
	    <Balance<T>>::insert(&second_asset_id.clone(), &caller.clone(), &account_data);
	    PolkadexSwap::<T>::register_swap_pair(RawOrigin::Signed(caller.clone()).into(), first_asset_id, second_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT));
	}: _(RawOrigin::Signed(caller.clone()), first_asset_id, second_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT))

	verify {
        assert_last_event::<T>(Event::<T>::LiqudityAdded(caller, first_asset_id, second_asset_id).into());
	}

	remove_liquidity{
	    let caller: T::AccountId = account("caller", 0, SEED);
        let first_asset_id = (1 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let second_asset_id = (2 as u64, caller.clone()).using_encoded(<T as frame_system::Config>::Hashing::hash);
        let account_data = polkadex_custom_assets::AccountData {
	        free_balance: PolkadexSwap::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	    <Balance<T>>::insert(&first_asset_id.clone(), &caller.clone(), &account_data);
	    <Balance<T>>::insert(&second_asset_id.clone(), &caller.clone(), &account_data);
	    PolkadexSwap::<T>::register_swap_pair(RawOrigin::Signed(caller.clone()).into(), first_asset_id, second_asset_id, T::Balance::from(UNIT), T::Balance::from(UNIT));
	}: _(RawOrigin::Signed(caller.clone()), first_asset_id, second_asset_id, T::Balance::from(UNIT/10))

	verify {
        assert_last_event::<T>(Event::<T>::LiqudityRemoved(caller, first_asset_id, second_asset_id).into());
	}

}

#[cfg(test)]
mod tests {
    use frame_support::assert_ok;
    use crate::mock::*;
    use super::*;

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_register_swap_pair::<Test>());
        });

        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_swap_with_exact_supply::<Test>());
        });

        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_swap_with_exact_target::<Test>());
        });

        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_add_liquidity::<Test>());
        });

        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_remove_liquidity::<Test>());
        });
    }
}
