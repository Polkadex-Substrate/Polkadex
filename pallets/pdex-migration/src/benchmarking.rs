//! Benchmarking setup for pallet-template

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelisted_caller};
use frame_support::{assert_ok, pallet_prelude::*, traits::Get};
use frame_system::{pallet_prelude::*, RawOrigin};
use rand::{RngCore, SeedableRng};
// use frame_system::Origin;
use sp_core::H256;
use sp_runtime::{traits::BlockNumberProvider, SaturatedConversion};

use crate::pallet::{Call, Config, Pallet as PDEXMigration, Pallet, *};

use super::*;

// use crate::pallet::Call as PDEXMigration;
// use crate::mock::{
//     // PDEXMigration,
//     PDEX
// };
pub const PDEX: u128 = 1000_000_000_000;
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
		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let mut random_slice = [0u8; 32];
		let mut rng = rand::rngs::StdRng::seed_from_u64(5 as u64);
		rng.fill_bytes(&mut random_slice);
		let mut eth_hash: T::Hash = T::Hash::default();
		eth_hash.as_mut().copy_from_slice(&random_slice);


			assert_ok!(PDEXMigration::<T>::set_migration_operational_status(RawOrigin::Root.into(),true));
			// Register relayers
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer1.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer2.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer3.clone(),true));

			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer1).into(), beneficiary.clone(),amount,eth_hash));
			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer2).into(), beneficiary.clone(),amount,eth_hash));

	}: _(RawOrigin::Signed(relayer3),beneficiary,amount,eth_hash.into())


	unlock {
		let relayer1 : T::AccountId = account("relayer1",0,0);
		let relayer2  : T::AccountId = account("relayer2",0,0);
		let relayer3 : T::AccountId = account("relayer3",0,0);
		let beneficiary : T::AccountId  = whitelisted_caller();

		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let mut random_slice = [0u8; 32];
		let mut rng = rand::rngs::StdRng::seed_from_u64(5 as u64);
		rng.fill_bytes(&mut random_slice);
		let mut eth_hash: T::Hash = T::Hash::default();
		eth_hash.as_mut().copy_from_slice(&random_slice);

		   assert_ok!(PDEXMigration::<T>::set_migration_operational_status(RawOrigin::Root.into(),true));
			// Register relayers
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer1.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer2.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer3.clone(),true));

		  assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer1).into(), beneficiary.clone(),amount,eth_hash));
			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer2).into(), beneficiary.clone(),amount,eth_hash));
			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer3).into(), beneficiary.clone(),amount,eth_hash));

		frame_system::Pallet::<T>::set_block_number(frame_system::Pallet::<T>::current_block_number()+T::LockPeriod::get());

	}: _(RawOrigin::Signed(beneficiary))

	remove_minted_tokens {
		let relayer1: T::AccountId = account("relayer1",0,0);
		let relayer2  : T::AccountId = account("relayer2",0,0);
		let relayer3 : T::AccountId = account("relayer3",0,0);
		let beneficiary: T::AccountId  = whitelisted_caller();
	  let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let mut random_slice = [0u8; 32];
		let mut rng = rand::rngs::StdRng::seed_from_u64(5 as u64);
		rng.fill_bytes(&mut random_slice);
		let mut eth_hash: T::Hash = T::Hash::default();
		eth_hash.as_mut().copy_from_slice(&random_slice);

		   assert_ok!(PDEXMigration::<T>::set_migration_operational_status(RawOrigin::Root.into(),true));
			// Register relayers
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer1.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer2.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer3.clone(),true));

		  assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer1).into(), beneficiary.clone(),amount,eth_hash));
			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer2).into(), beneficiary.clone(),amount,eth_hash));
			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer3).into(), beneficiary.clone(),amount,eth_hash));

	}: _(RawOrigin::Root,beneficiary)
}
#[cfg(test)]
mod tests {
	use crate::mock::*;

	use super::*;

	impl_benchmark_test_suite!(Template, crate::mock::new_test_ext(), crate::mock::Test,);
}
