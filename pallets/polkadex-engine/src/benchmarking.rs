#![cfg(feature = "runtime-benchmarks")]

use codec::Decode;
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use frame_support::traits::Box;

use super::*;
const UNIT: u32 = 1_000_000;

fn set_up_asset_id_token<T: Config>(who: T::AccountId,
                                    total_issuance: T::Balance,
                                    minimum_deposit: T::Balance) -> T::Hash {
    polkadex_custom_assets::Module::<T>::create_token(RawOrigin::Signed(who).into(), total_issuance, minimum_deposit);
    polkadex_custom_assets::Module::<T>::get_asset_id()[2]
}

benchmarks! {
    register_new_orderbook_with_polkadex {
		let caller: T::AccountId = polkadex_custom_assets::Module::<T>::get_account_id()[0].clone();
		let asset_id = set_up_asset_id_token::<T>(caller.clone(), T::Balance::from(10*UNIT), T::Balance::from(0));
    }: _(RawOrigin::Signed(caller), asset_id, T::Balance::from(UNIT))

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
    }
}

