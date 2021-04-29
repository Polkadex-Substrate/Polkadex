//! Benchmarking setup for pallet-template
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_system::{EventRecord, RawOrigin};
use frame_benchmarking::{benchmarks, whitelisted_caller, impl_benchmark_test_suite};
use crate::Pallet as PolkadexIdo;
use orml_tokens::{AccountData, Accounts};
use sp_runtime::traits::Bounded;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

benchmarks! {
	register_investor {
		let caller: T::AccountId = whitelisted_caller();
		let currency_id: T::CurrencyId = T::NativeCurrencyId::get();
		let account_data: AccountData<T::Balance> = AccountData{
			free: T::Balance::max_value(),
			reserved: T::Balance::zero(),
			frozen: T::Balance::zero()
		};

		<Accounts<T>>::insert(caller.clone(), currency_id, account_data);

	}: _(RawOrigin::Signed(caller.clone()))
	verify {
		assert_last_event::<T>(Event::<T>::InvestorRegistered(caller).into());
	}
}

impl_benchmark_test_suite!(
	PolkadexIdo,
	crate::mock::ExtBuilder::default().build(),
	crate::mock::Test,
);
