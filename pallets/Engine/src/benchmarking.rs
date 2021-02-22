#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_system::{EventRecord, RawOrigin};
use frame_support::ensure;
use frame_benchmarking::{benchmarks, account, whitelisted_caller};
use crate::Module as Identity;
const SEED: u32 = 0;

benchmarks! {

	settle_trade {
	    let caller: T::AccountId = whitelisted_caller();
	    let maker = Order{
	    price,
        quantity,
        order_type,
        trader,
        nonce,
        asset_id,
        signature,
	    };
	    let taker = Order{
	    price,
        quantity,
        order_type,
        trader,
        nonce,
        asset_id,
        signature,
	    }
	}: _(RawOrigin::Signed(caller), maker, taker)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{new_test_ext, Test};
    use frame_support::assert_ok;

    #[test]
    fn test_benchmarks() {
        new_test_ext().execute_with(|| {
            assert_ok!(test_benchmark_settle_trade::<Test>());
        });
    }
}