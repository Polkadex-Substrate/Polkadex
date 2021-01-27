#![cfg(feature = "runtime-benchmarks")]

use super::*;
use sp_std::prelude::*;
use frame_system::RawOrigin;
use frame_support::{ensure, traits::OnFinalize,};
use frame_benchmarking::{benchmarks, TrackedStorageKey, account};

const SEED: u32 = 0;

benchmarks! {

	add_registrar {
		let account_id = account("registrar", 0, SEED);
	}: _(RawOrigin::Root, account_id)

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

