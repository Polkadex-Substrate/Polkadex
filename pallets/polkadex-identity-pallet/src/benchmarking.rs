#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_system::{EventRecord, RawOrigin};
use frame_support::ensure;
use frame_benchmarking::{benchmarks, account, whitelisted_caller};
use crate::Module as Identity;
const SEED: u32 = 0;

fn add_sub_accounts<T: Config>(who: &T::AccountId, s: u32) -> Result<Vec<T::AccountId>, &'static str> {
    let mut subs: Vec<T::AccountId> = Vec::new();
    for i in 0..s {
        let sub_account: T::AccountId = account("sub", i, SEED);
        subs.push(sub_account.clone());
    }
    <SubsOf<T>>::insert(who, subs.clone());
    Ok(subs)
}

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
	    ensure!(Registrars::<T>::contains_key(&account), "Registrars not added.");
		assert_last_event::<T>(Event::<T>::RegistrarAdded(account).into());
	}

	provide_judgement_trader {
	    let caller: T::AccountId = whitelisted_caller();
        let account: T::AccountId = account("new", 0, SEED);
        Identity::<T>::add_registrar(RawOrigin::Root.into(), caller.clone())?;
	}: _(RawOrigin::Signed(caller), account.clone(), Judgement::Reasonable)
	verify {
		assert_last_event::<T>(Event::<T>::JudgementGiven(account).into());
	}

	add_sub_account {
        let s in 1 .. T::MaxSubAccounts::get() - 1;

		let caller: T::AccountId = whitelisted_caller();
		let _ = add_sub_accounts::<T>(&caller, s)?;
		let sub = account("new_sub", 0, SEED);
        <IdentityOf<T>>::insert(&caller, Judgement::Reasonable);
		ensure!(SubsOf::<T>::get(&caller).len() as u32 == s, "Subs not set.");
	}: _(RawOrigin::Signed(caller.clone()), sub)
	verify {
		ensure!(SubsOf::<T>::get(&caller).len() as u32 == s + 1, "Subs not added.");
		assert_last_event::<T>(Event::<T>::SubIdentityAdded.into());
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
            assert_ok!(test_benchmark_provide_judgement_trader::<Test>());
            assert_ok!(test_benchmark_add_sub_account::<Test>());
        });
    }
}

