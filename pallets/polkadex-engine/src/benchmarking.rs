#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::ensure;
use frame_system::{EventRecord, RawOrigin};
use sp_core::H256;

use super::*;
const UNIT: u128 = 1_000_000_000_000;

pub fn u128_to_balance(input: u128) -> T::Balance {
    input.into()
}

fn set_up_asset_id_token<T: Config>() {
    let endowed_accounts: Vec<u64> = vec![1, 2];
    let mut genesis = system::GenesisConfig::default().build_storage::<Module<T>>().unwrap();
    let temp = T::Hashing::hash_of(&H256::from_low_u64_be(8u64));
    let temp2 = T::Hashing::hash_of(&H256::from_low_u64_be(10u64));
    polkadex_custom_assets::GenesisConfig::<T> {
        native_asset: T::Hashing::hash_of(&H256::zero()),
        assets: vec![T::Hashing::hash_of(&H256::zero()), temp, temp2],
        initial_balance: <Module<T>>::convert_balance_to_fixed_u128(u128_to_balance(1000 * UNIT)).unwrap(),
        endowed_accounts: endowed_accounts
            .clone().into_iter().map(Into::into).collect(),
    }.assimilate_storage(&mut genesis).unwrap();
}

benchmarks! {
    register_new_orderbook_with_polkadex {
        let caller: T::AccountId = whitelisted_caller();
        //set_up_asset_id_token::<T>(caller.clone(), T::Balance::from(10*UNIT), T::Balance::from(0));
        let quote_asset_id = T::Hashing::hash_of(&(0 as u64, caller.clone(), u128_to_balance(10*UNIT)));
    }: _(RawOrigin::Signed(caller), quote_asset_id, u128_to_balance(UNIT))

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

