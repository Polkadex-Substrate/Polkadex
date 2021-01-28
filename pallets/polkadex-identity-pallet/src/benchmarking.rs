#![cfg(feature = "runtime-benchmarks")]

use super::*;
use sp_std::prelude::*;
use frame_system::{EventRecord, RawOrigin};
use frame_support::{ensure, traits::OnFinalize,};
use frame_benchmarking::{benchmarks, TrackedStorageKey, account};

const SEED: u32 = 0;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Module::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

benchmarks! {

	add_registrar {
		let account: T::AccountId = account("registrar", 0, SEED);
	}: _(RawOrigin::Root, account.clone())
	verify {
	    ensure!(Registrars::<T>::get(&account) == Judgement::PolkadexFoundationAccount, "RegistrarAlreadyPresent");
		assert_last_event::<T>(Event::<T>::RegistrarAdded(account).into());
	}

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_add_registrar::<Test>());
        });
    }
}

