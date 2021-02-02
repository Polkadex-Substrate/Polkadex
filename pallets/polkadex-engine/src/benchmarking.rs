#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};

use super::*;

benchmarks! {
    register_new_orderbook_with_polkadex {
        let caller: T::AccountId = whitelisted_caller();
        let mut quote_asset_id = T::Hash::default();
        let amount = T::Balance::from(100u32);
    }: _(RawOrigin::Signed(caller), quote_asset_id.clone(), amount)

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

