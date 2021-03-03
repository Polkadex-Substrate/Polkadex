#![cfg(feature = "runtime-benchmarks")]
use frame_support::traits::Vec;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use sp_std::collections::btree_map;
use sp_runtime::{traits::AccountIdConversion, AccountId32};
use crate::Module as Identity;
use super::*;
use crate::types::OrderType;
use sp_io::crypto;
use sp_core::{H256};
use sp_std::boxed::Box;
const SEED: u32 = 0;

benchmarks! {

	settle_trade {
	    let caller: T::AccountId = whitelisted_caller();
	    // Add caller to Providers
	    Providers::<T>::insert(&caller,1 as u32);

	    // Credit Maker Account
            let pubkey_maker = crypto::sr25519_generate(sp_core::crypto::KeyTypeId::from(1), None);
	    let maker: T::AccountId = T::AccountId::decode(&mut &pubkey_maker[..]).unwrap_or_default();
	    let maker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    };
	    let maker_msg = (T::Balance::from(128u32), T::Balance::from(12u32), OrderType::BidLimit, 0 as u64).using_encoded(<T as frame_system::Config>::Hashing::hash);
	    Traders::<T>::insert(&maker, maker_acc);

	    // Credit Taker Account
	    let pubkey_taker = crypto::sr25519_generate(sp_core::crypto::KeyTypeId::from(2), None);
	    let taker: T::AccountId = T::AccountId::decode(&mut &pubkey_taker[..]).unwrap_or_default();
	    let taker_acc: AccountData<T::Hash,T::Balance> = AccountData{
	        nonce: 0,
            assets: btree_map::BTreeMap::new()
	    };
	    let taker_msg = (T::Balance::from(128u32), T::Balance::from(12u32), OrderType::AskLimit, 0 as u64).using_encoded(<T as frame_system::Config>::Hashing::hash);
	    Traders::<T>::insert(&taker, taker_acc);

	    let maker_order = Order{
	    price: T::Balance::from(128u32),
        quantity: T::Balance::from(12u32),
        order_type: OrderType::BidLimit,
        trader: maker,
        nonce: 0,
        asset_id: T::Hash::default(),
        signature: sp_io::crypto::sr25519_sign(sp_core::crypto::KeyTypeId::from(1),&pubkey_maker,maker_msg.as_ref()).unwrap().0.encode(),
	    };

	    let taker_order = Order{
	    price: T::Balance::from(128u32),
        quantity: T::Balance::from(12u32),
        order_type: OrderType::AskLimit,
        trader: taker,
        nonce:0,
        asset_id: T::Hash::default(),
        signature: sp_io::crypto::sr25519_sign(sp_core::crypto::KeyTypeId::from(2),&pubkey_taker,taker_msg.as_ref()).unwrap().0.encode(),
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
}
