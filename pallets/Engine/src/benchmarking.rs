#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use sp_std::collections::btree_map;
use codec::{Decode, Encode};
use crate::Module as Identity;
use super::*;
use crate::types::OrderType;
use sp_core::H256;

const SEED: u32 = 0;

benchmarks! {

	settle_trade {
	    let caller: T::AccountId = whitelisted_caller();
	    // Add caller to Providers
	    <Providers<T>>::insert(caller,Some(1u32).into());
	    // Credit Maker Account
	    let maker: T::AccountId = account("maker", 0, SEED);
	    let maker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    };
	    <Traders<T>>::insert(maker,maker_acc);
	    // Credit Taker Account
	    let taker: T::AccountId = account("taker", 0, SEED);
	    let taker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    };
	    let price: T::Balance = T::Balance::from(128 as u32);
	    let quantity: T::Balance = T::Balance::from(1 as u32);
	    let asset_id: T::Hash = T::Hash::default();
	    <Traders<T>>::insert(taker, taker_acc);
	    let signature: T::Signature = ();
	    let maker_order: Order<T::Balance, T::AccountId, T::Hash, T::Signature> = Order{
	                        price,
	                        quantity,
	                        order_type: OrderType::BidLimit,
	                        trader: maker,
	                        nonce: 0,
	                        asset_id,
	                        signature,
	                    };
	    let taker_order: Order<T::Balance, T::AccountId, T::Hash, T::Signature> = Order{
	                        price,
	                        quantity,
	                        order_type: OrderType::AskLimit,
	                        trader: maker,
	                        nonce: 0,
	                        asset_id,
	                        signature,
	                    };

	}: _(RawOrigin::Signed(caller), maker_order, taker_order)
}

#[cfg(test)]
mod tests {
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

    /*    #[test]
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
        }*/
}