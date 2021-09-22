//! Benchmarking setup for pallet-template

use super::*;

use frame_benchmarking::{benchmarks,account, whitelisted_caller, impl_benchmark_test_suite};
use sp_runtime::traits::BlockNumberProvider;
#[allow(unused)]
// use crate::pallet::Call as PDEXMigration;
// use crate::mock::{
//     // PDEXMigration,
//     PDEX
// };
use crate::pallet as pdex_migration;

use crate::pallet::{Config,Call,Pallet};
use frame_support::traits::Get;
use sp_runtime::traits::Saturating;
use frame_support::assert_ok;
// use frame_system::Origin;
use sp_core::H256;
use frame_support::pallet_prelude::*;
use frame_system::pallet_prelude::*;
use frame_system::Origin;
use frame_system::RawOrigin;



use Pallet as PDEXMigration;

use crate::pallet::*;

pub const PDEX:u128 = 1000_000_000_000;

benchmarks! {
    set_migration_operational_status {

    }: _(RawOrigin::Root, true)

    set_relayer_status {
        let relayer : T::AccountId = account("relayer",0,0);
    }: _ (RawOrigin::Root, relayer, true)

    mint {
        let relayer1: T::AccountId = account("relayer1",0,0);
        let relayer2: T::AccountId = account("relayer2",0,0);
        let relayer3: T::AccountId = account("relayer3",0,0);
        let beneficiary: T::AccountId  = whitelisted_caller();
        #[cfg(test)]
        {
            assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(),true));
            // Register relayers
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer1,true));
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer2,true));
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer3,true));

            assert_ok!(PDEXMigration::mint(Origin::Signed(relayer1), beneficiary,100*PDEX,H256::zero()));
            assert_ok!(PDEXMigration::mint(Origin::Signed(relayer2), beneficiary,100*PDEX,H256::zero()));
        }
        let beneficiary: T::AccountId = whitelisted_caller();
        let eth_hash: T::Hash = T::Hash::default();
        let relayer3: T::AccountId = account("relayer3",0,0);
        let amount: T::Balance = <T as pallet_balances::Config>::ExistentialDeposit::get().saturating_mul(100u32.into());
    }: _(RawOrigin::Signed(relayer3),beneficiary,amount,eth_hash)


    unlock {
        let relayer1 : T::AccountId = account("relayer1",0,0);
        let relayer2  : T::AccountId = account("relayer2",0,0);
        let relayer3 : T::AccountId = account("relayer3",0,0);
        let beneficiary : T::AccountId  = whitelisted_caller();
        #[cfg(test)]
        {
            assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(),true));
            // Register relayers
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer1,true));
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer2,true));
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer3,true));

            assert_ok!(PDEXMigration::mint(Origin::signed(relayer1), beneficiary,100*PDEX,H256::zero()));
            assert_ok!(PDEXMigration::mint(Origin::signed(relayer2), beneficiary,100*PDEX,H256::zero()));
            assert_ok!(PDEXMigration::mint(Origin::signed(relayer3), beneficiary,100*PDEX,H256::zero()));
        }
        frame_system::Pallet::<T>::set_block_number(frame_system::Pallet::<T>::current_block_number()+T::LockPeriod::get());

        let beneficiary: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Signed(beneficiary))

    remove_minted_tokens {
        let relayer1: T::AccountId = account("relayer1",0,0);
        let relayer2  : T::AccountId = account("relayer2",0,0);
        let relayer3 : T::AccountId = account("relayer3",0,0);
        let beneficiary: T::AccountId  = whitelisted_caller();
        #[cfg(test)]
        {
            assert_ok!(PDEXMigration::set_migration_operational_status(Origin::root(),true));
            // Register relayers
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer1,true));
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer2,true));
            assert_ok!(PDEXMigration::set_relayer_status(Origin::root(),relayer3,true));

            assert_ok!(PDEXMigration::mint(Origin::signed(relayer1), beneficiary,100*PDEX,H256::zero()));
            assert_ok!(PDEXMigration::mint(Origin::signed(relayer2), beneficiary,100*PDEX,H256::zero()));
            assert_ok!(PDEXMigration::mint(Origin::signed(relayer3), beneficiary,100*PDEX,H256::zero()));
        }
        let beneficiary: T::AccountId = whitelisted_caller();
    }: _(RawOrigin::Root,beneficiary)
}
#[cfg(test)]
mod tests {
	use super::*;
	use crate::mock::*;
    impl_benchmark_test_suite!(
        Template,
        crate::mock::new_test_ext(),
        crate::mock::Test,
    );
}