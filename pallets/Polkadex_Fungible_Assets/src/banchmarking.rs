#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks, account, whitelisted_caller};
use polkadex_primitives::assets::AssetId;
use sp_runtime::traits::StaticLookup;
const SEED: u32 = 0;

fn set_balance<T: Config>(asset_id: AssetId, account_id: &T::AccountId, amount: T::Balance)
{
	let value = amount;
	Balances::<T>::insert(asset_id, &account_id, &value);
	TotalIssuance::<T>::insert(asset_id, value);
}

benchmarks! {
	transfer {
		let asset_id = AssetId::POLKADEX;
		let alice: T::AccountId = whitelisted_caller();
        let bob: T::AccountId = account("bob", 1, SEED);
        set_balance::<T>(asset_id, &alice.clone(), T::Balance::from(500u32));
		let recipient_lookup: <T::Lookup as StaticLookup>::Source = T::Lookup::unlookup(bob.clone());

	}: _(RawOrigin::Signed(alice), asset_id, recipient_lookup, T::Balance::from(100u32))
}

#[cfg(test)]
mod tests {
	use frame_support::assert_ok;
	use crate::mock::{new_tester, Test};
	use super::*;

	#[test]
	fn test_benchmarks() {
		new_tester().execute_with(|| {
			assert_ok!(test_benchmark_transfer::<Test>());
		});
	}
}
