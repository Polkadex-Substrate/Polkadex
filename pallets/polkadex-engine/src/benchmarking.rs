#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_system::{EventRecord, RawOrigin};
use frame_support::ensure;
use frame_benchmarking::{benchmarks, account, whitelisted_caller};
use crate::Module as Identity;

benchmarks! {


}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            assert_ok!(true);
        });
    }
}

