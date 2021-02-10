#![cfg(feature = "runtime-benchmarks")]

use codec::Decode;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use frame_support::traits::Box;
use crate::Module as Polkadex;
use polkadex_custom_assets::Balance;
use sp_std::vec::Vec;
use sp_core::H256;
use super::*;
use sp_std::prelude::*;

const UNIT: u32 = 1_000_000;

fn set_up_asset_id_token<T: Config>() -> (T::AccountId, T::Hash) {
    let who: T::AccountId = polkadex_custom_assets::Module::<T>::get_account_id();
    polkadex_custom_assets::Module::<T>::create_token(RawOrigin::Signed(who.clone()).into(), T::Balance::from(10*UNIT), T::Balance::from(0));
    (who, polkadex_custom_assets::Module::<T>::get_asset_id())
}

fn set_account_with_fund<T: Config>(sender: T::AccountId) -> (T::Hash) {
    let base_asset_id: T::Hash = T::Hashing::hash_of(&(1 as u64, sender.clone(),T::Balance::from(10*UNIT)));
    let account_data = polkadex_custom_assets::AccountData {
        free_balance: Polkadex::<T>::convert_balance_to_fixed_u128(T::Balance::from(1000 * UNIT)).unwrap(),
        reserved_balance: FixedU128::from(0),
        misc_frozen: FixedU128::from(0),
        fee_frozen: FixedU128::from(0),
    };
    <Balance<T>>::insert(&base_asset_id.clone(), &sender.clone(), &account_data);
    base_asset_id
}

fn set_up_bulk_order<T: Config>(sender: T::AccountId, trading_pair: (T::Hash, T::Hash)) {
    let price_levels: u32 = 5001;
    let orders_per_level: u32 = 1001;
    let order_amount: FixedU128 = FixedU128::from(100);
    for price in 1..price_levels{
        for order in 1..orders_per_level{
            Polkadex::<T>::execute_order(sender.clone(),
                                         OrderType::AskLimit,trading_pair,Polkadex::<T>::convert_balance_to_fixed_u128(T::Balance::from(price * 1000)).unwrap(), order_amount);
        }
    }
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
		let (caller, quote_asset_id): (T::AccountId, T::Hash) = set_up_asset_id_token::<T>();
        let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
		let trading_pair_id = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency);

    }: _(RawOrigin::Signed(caller), quote_asset_id, T::Balance::from(UNIT))
    verify {
		assert_last_event::<T>(Event::<T>::TradingPairCreated(trading_pair_id.0, trading_pair_id.1).into());
	}

	register_new_orderbook {
	    let (caller, quote_asset_id): (T::AccountId, T::Hash) = set_up_asset_id_token::<T>();
		let base_asset_id: T::Hash = set_account_with_fund::<T>(caller.clone());
		let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
		let trading_pair_id1 = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency);
		Polkadex::<T>::create_order_book(trading_pair_id1.0, trading_pair_id1.1, trading_pair_id1);
		let trading_pair_id2 = Polkadex::<T>::get_pair(base_asset_id.clone(), native_currency);
		Polkadex::<T>::create_order_book(trading_pair_id2.0, trading_pair_id2.1, trading_pair_id2);
        let trading_pair_id = Polkadex::<T>::get_pair(quote_asset_id.clone(), base_asset_id.clone());

	}: _(RawOrigin::Signed(caller), quote_asset_id.clone(), T::Balance::from(UNIT), base_asset_id.clone(), T::Balance::from(UNIT))
	verify {
		assert_last_event::<T>(Event::<T>::TradingPairCreated(trading_pair_id.0, trading_pair_id.1).into());
	}

	submit_order {
	    let (caller, quote_asset_id): (T::AccountId, T::Hash) = set_up_asset_id_token::<T>();
	    let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
		let trading_pair_id1 = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency.clone());
		Polkadex::<T>::create_order_book(trading_pair_id1.0, trading_pair_id1.1, trading_pair_id1);
		let caller2: T::AccountId = whitelisted_caller();
        let base_asset_id = set_account_with_fund::<T>(caller2.clone());
	    let trading_pair_id2 = Polkadex::<T>::get_pair(base_asset_id.clone(), native_currency.clone());
		set_up_bulk_order::<T>(caller2, trading_pair_id2);
	}: _(RawOrigin::Signed(caller), OrderType::BidLimit, (quote_asset_id.clone(),
	native_currency.clone()), T::Balance::from(1000 * 5000), T::Balance::from(1000 * 5000))

	cancel_order {
	    let (caller, quote_asset_id): (T::AccountId, T::Hash) = set_up_asset_id_token::<T>();
	    let native_currency = polkadex_custom_assets::PolkadexNativeAssetIdProvider::<T>::asset_id();
	    let trading_pair_id1 = Polkadex::<T>::get_pair(quote_asset_id.clone(), native_currency.clone());
		Polkadex::<T>::create_order_book(trading_pair_id1.0, trading_pair_id1.1, trading_pair_id1);
        set_up_bulk_order::<T>(caller.clone(), trading_pair_id1);
        let linked_pricelevel: LinkedPriceLevel<T> = <PriceLevels<T>>::get(&trading_pair_id1, &Polkadex::<T>::convert_balance_to_fixed_u128(T::Balance::from(4999)).unwrap());
        let order: Order<T> = linked_pricelevel.orders[4000].clone();

	}: _(RawOrigin::Signed(caller), order.id, (quote_asset_id.clone(),
	native_currency.clone()), T::Balance::from(5000))

}

#[cfg(test)]
mod tests {
    use frame_support::assert_ok;
    use crate::mock::*;
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

