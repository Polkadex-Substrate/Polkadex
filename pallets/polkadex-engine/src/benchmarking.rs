#![cfg(feature = "runtime-benchmarks")]

use codec::Decode;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use frame_support::traits::Box;
use crate::Module as Polkadex;
use polkadex_custom_assets::Balance;

use sp_core::H256;
use super::*;
const UNIT: u32 = 1_000_000;

fn set_up_asset_id_token<T: Config>(who: T::AccountId,
                                    total_issuance: T::Balance,
                                    minimum_deposit: T::Balance) -> T::Hash {
    polkadex_custom_assets::Module::<T>::create_token(RawOrigin::Signed(who).into(), total_issuance, minimum_deposit);
    polkadex_custom_assets::Module::<T>::get_asset_id()
}

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Module::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

benchmarks! {
    register_new_orderbook_with_polkadex {
		let caller: T::AccountId = polkadex_custom_assets::Module::<T>::get_account_id();
		let quote_asset_id = set_up_asset_id_token::<T>(caller.clone(), T::Balance::from(10*UNIT), T::Balance::from(0));
        let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
		let trading_pair_id = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency);

    }: _(RawOrigin::Signed(caller), quote_asset_id, T::Balance::from(UNIT))
    verify {
		assert_last_event::<T>(Event::<T>::TradingPairCreated(trading_pair_id.0, trading_pair_id.1).into());
	}

	register_new_orderbook {
	    let caller: T::AccountId = polkadex_custom_assets::Module::<T>::get_account_id();
		let quote_asset_id = set_up_asset_id_token::<T>(caller.clone(), T::Balance::from(10*UNIT), T::Balance::from(0));
		let alice: u64 = 1;
		let base_asset_id = T::Hashing::hash_of(&(1 as u64, alice.clone(),T::Balance::from(10*UNIT)));
		let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
		let trading_pair_id1 = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency);
		Polkadex::<T>::create_order_book(trading_pair_id1.0, trading_pair_id1.1, trading_pair_id1);
		let trading_pair_id2 = Polkadex::<T>::get_pair(base_asset_id.clone(), native_currency);
		Polkadex::<T>::create_order_book(trading_pair_id2.0, trading_pair_id2.1, trading_pair_id2);
        let trading_pair_id = Polkadex::<T>::get_pair(quote_asset_id.clone(), base_asset_id.clone());
		let account_data = polkadex_custom_assets::AccountData {
	        free_balance: Polkadex::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	   <Balance<T>>::insert(&base_asset_id.clone(), &caller.clone(), &account_data);
	}: _(RawOrigin::Signed(caller), quote_asset_id.clone(), T::Balance::from(UNIT), base_asset_id.clone(), T::Balance::from(UNIT))
	verify {
		assert_last_event::<T>(Event::<T>::TradingPairCreated(trading_pair_id.0, trading_pair_id.1).into());
	}

	submit_order {
	    let caller: T::AccountId = polkadex_custom_assets::Module::<T>::get_account_id();
	    let quote_asset_id = set_up_asset_id_token::<T>(caller.clone(), T::Balance::from(10*UNIT), T::Balance::from(0));
	    let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
		let trading_pair_id1 = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency.clone());
		Polkadex::<T>::create_order_book(trading_pair_id1.0, trading_pair_id1.1, trading_pair_id1);

	}: _(RawOrigin::Signed(caller), OrderType::BidMarket, (quote_asset_id.clone(),
	native_currency.clone()), T::Balance::from(1000 * UNIT), T::Balance::from(1000 * UNIT))

	cancel_order {
	    let caller: T::AccountId = polkadex_custom_assets::Module::<T>::get_account_id();
	    let quote_asset_id = set_up_asset_id_token::<T>(caller.clone(), T::Balance::from(10*UNIT), T::Balance::from(0));
	    let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
	    let trading_pair_id1 = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency.clone());
		Polkadex::<T>::create_order_book(trading_pair_id1.0, trading_pair_id1.1, trading_pair_id1);

	}: _(RawOrigin::Signed(caller), T::Hash::default(), (quote_asset_id.clone(),
	native_currency.clone()), T::Balance::from(1000 * UNIT))

}

#[cfg(test)]
mod tests {
    use frame_support::assert_ok;
    use crate::mock::{new_test_ext, Test};
    use super::*;

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_register_new_orderbook_with_polkadex::<Test>());
        });
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_register_new_orderbook::<Test>());
        });
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_submit_order::<Test>());
        });
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_cancel_order::<Test>());
        });
    }
}

