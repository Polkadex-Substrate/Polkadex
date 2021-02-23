#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;
use sp_std::collections::btree_map;
use sp_std::prelude::*;
use sp_std::vec::Vec;
use super::*;
use crate::types::OrderType;

const SEED: u32 = 0;

benchmarks! {

	settle_trade {
	    let caller: T::AccountId = whitelisted_caller();
	    // Add caller to Providers
	    Providers::<T>::insert(&caller,1 as u32);

	    // Credit Maker Account
	    let (makerPair, _) = sp_core::sr25519::Pair::generate();
	    let pubkey_maker = makerPair.public().0;
	    let maker: T::AccountId = T::AccountId::decode(&mut &pubkey_maker[..]).unwrap_or_default();
	    let maker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    };
	    let maker_msg = (T::Balance::from(128), T::Balance::from(12), OrderType::BidLimit, 0 as u64).using_encoded(<T as frame_system::Config>::Hashing::hash);
	    Traders::<T>::insert(&maker, maker_acc);

	    // Credit Taker Account
	    let (takerPair, _) = sp_core::sr25519::Pair::generate();
	    let pubkey_taker = takerPair.public().0;
	    let taker: T::AccountId = T::AccountId::decode(&mut &pubkey_taker[..]).unwrap_or_default();
	    let taker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    };
	    let taker_msg = (T::Balance::from(128), T::Balance::from(12), OrderType::AskLimit, 0 as u64).using_encoded(<T as frame_system::Config>::Hashing::hash);
	    Traders::<T>::insert(&taker, taker_acc);

	    let maker_order = Order{
	    price: T::Balance::from(128),
        quantity: T::Balance::from(12),
        order_type: OrderType::BidLimit,
        trader: maker,
        nonce: 0,
        asset_id: T::Hash::default(),
        signature: makerPair.sign(maker_msg.as_ref()).encode(),
	    };

	    let taker_order = Order{
	    price: T::Balance::from(128),
        quantity: T::Balance::from(12),
        order_type: OrderType::AskLimit,
        trader: taker,
        nonce:0,
        asset_id: T::Hash::default(),
        signature: takerPair.sign(taker_msg.as_ref()).encode(),
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
