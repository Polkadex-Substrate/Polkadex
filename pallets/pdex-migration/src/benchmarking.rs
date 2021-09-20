//! Benchmarking setup for pallet-template

use super::*;

use frame_system::RawOrigin;
use frame_benchmarking::{benchmarks,account, whitelisted_caller, impl_benchmark_test_suite};
#[allow(unused)]
use crate::pallet as PDEXMigration;

use crate::pallet::{Config,Call,Pallet};
use frame_support::traits::Get;
use sp_runtime::traits::Saturating;

// 3  | use crate::pallet::Pallet;
//    |
// 3  | use frame_system::Pallet;
benchmarks! {
    set_migration_operational_status {

    }: _(RawOrigin::Root, true)

    set_relayer_status {
        let relayer: T::AccountId = account("relayer",0,0);
    }: _ (RawOrigin::Root, relayer, true)

    mint {
        let beneficiary: T::AccountId = whitelisted_caller();
        let eth_hash: T::Hash = T::Hash::default();
        let relayer3: T::AccountId = account("relayer3",0,0);
        let amount: T::Balance = <T as pallet_balances::Config>::ExistentialDeposit::get().saturating_mul(100u32.into());
    }: _(RawOrigin::Signed(relayer3),beneficiary,amount,eth_hash)

    unlock {
        

        let beneficiary: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(beneficiary))
    remove_minted_tokens {
        let beneficiary: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Root,beneficiary)
}

impl_benchmark_test_suite!(
	Template,
	crate::mock::new_test_ext(),
	crate::mock::Test,
);
