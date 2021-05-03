//! Benchmarking setup for pallet-template


use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account};
use frame_support::{
	traits::{Get, EnsureOrigin, UnfilteredDispatchable},
};
use frame_system::{self, EventRecord, RawOrigin};
use orml_tokens::{AccountData, Accounts};
use sp_runtime::traits::Bounded;
use sp_runtime::traits::One;

use crate::Pallet as PolkadexIdo;

use super::*;

const SEED: u32 = 0;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

fn set_up<T: Config>(caller: T::AccountId) {
	let currency_id: T::CurrencyId = T::NativeCurrencyId::get();
	let account_data: AccountData<T::Balance> = AccountData{
		free: T::Balance::max_value(),
		reserved: T::Balance::zero(),
		frozen: T::Balance::zero()
	};

	<Accounts<T>>::insert(caller, currency_id, account_data);
}

benchmarks! {
	register_investor {
		let caller: T::AccountId = account("origin", 0, SEED);
		set_up::<T>(caller.clone());
		whitelist_account!(caller);
	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert_last_event::<T>(Event::<T>::InvestorRegistered(caller).into());
	}

	attest_investor {
		let caller: T::AccountId = account("origin", 0, SEED);
		set_up::<T>(caller.clone());
		whitelist_account!(caller);
		PolkadexIdo::<T>::register_investor(RawOrigin::Signed(caller.clone()).into());
		let call = Call::<T>::attest_investor(caller.clone(), KYCStatus::Tier0);
		let origin = T::GovernanceOrigin::successful_origin();
	}: {call.dispatch_bypass_filter(origin)?}
	verify {
		assert_last_event::<T>(Event::<T>::InvestorAttested(caller).into());
	}

	register_round {
		let caller: T::AccountId = account("origin", 0, SEED);
		whitelist_account!(caller);
		let balance = T::Balance::one();
    	let block_num = T::BlockNumber::one();
	}: _(RawOrigin::Signed(caller.clone()),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                block_num,
                balance,
                balance,
                balance,
                balance,
                block_num)
	verify {
		ensure!(<InfoProjectTeam<T>>::contains_key(caller), "Register Funding Round didn't work");
	}

	whitelist_investor {
		let investor_address: T::AccountId = account("origin", 100, SEED);
		set_up::<T>(investor_address.clone());
		whitelist_account!(investor_address);
		PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
		let caller: T::AccountId = account("origin", 101, SEED);
		whitelist_account!(caller);
		let balance = T::Balance::one();
    	let block_num = T::BlockNumber::one();
		PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
		let round_id = <InfoProjectTeam<T>>::get(caller.clone());
	}: _(RawOrigin::Signed(caller.clone()), round_id, investor_address.clone(), T::Balance::max_value())
	verify {
		ensure!(<WhiteListInvestors<T>>::contains_key(&round_id, investor_address), "WhiteListInvestors didn't work");
	}
}

impl_benchmark_test_suite!(
	PolkadexIdo,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Test,
);
