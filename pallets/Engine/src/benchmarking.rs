#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use sp_std::collections::btree_map;

use crate::mock::EngineModule;
use crate::Module as Identity;

use super::*;
use crate::types::OrderType;
use sp_core::H256;

const SEED: u32 = 0;

benchmarks! {

	settle_trade {
	    let caller: T::AccountId = whitelisted_caller();
	    // Add caller to Providers
	    <Providers<T>>::insert(caller,Some(1));
	    // Credit Maker Account
	    let maker: T::AccountId = account("maker", 0, SEED);
	    let maker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    }
	    <Traders<T>>::insert(maker,maker_acc);
	    // Credit Taker Account
	    let taker: T::AccountId = account("taker", 0, SEED);
	    let taker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    }
	    <Traders<T>>::insert(taker, taker_acc);
	    let maker_order = Order{
	    128 as u128,
        1 as u128,
        OrderType::BidLimit,
        maker,
        nonce: 0,
        asset_id: H256::random(),
        signature: (),
	    };
	    let taker_order = Order{
	    128 as u128,
        1 as u128,
        OrderType::AskLimit,
        taker,
        nonce:0,
        asset_id: H256::random(),
        signature: (),
	    }
	}: _(RawOrigin::Signed(caller), maker_order, taker_order)
}

#[cfg(test)]
mod tests {
    use frame_support::assert_ok;

    use crate::mock::{new_test_ext, Test};

    use super::*;
    use crate::{Error, mock::*};
    use frame_support::{assert_ok, assert_noop};

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_settle_trade::<Test>());
        });
    }

    #[test]
    fn it_works_for_default_value() {
        new_test_ext().execute_with(|| {
            // // Dispatch a signed extrinsic.
            // assert_ok!(EngineModule::do_something(Origin::signed(1), 42));
            // // Read pallet storage and assert an expected result.
            // assert_eq!(EngineModule::something(), Some(42));
        });
    }

    #[test]
    fn correct_error_for_none_value() {
        new_test_ext().execute_with(|| {
            // Ensure the expected error is thrown when no value is present.
            // assert_noop!(
            // 	EngineModule::cause_error(Origin::signed(1)),
            // 	Error::<Test>::NoneValue
            // );
        });
    }
}