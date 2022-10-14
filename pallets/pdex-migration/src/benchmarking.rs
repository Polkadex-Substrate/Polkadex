use crate::pallet::{Call, Config, Pallet as PDEXMigration, Pallet, *};
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable, traits::Get};
use frame_system::RawOrigin;
use sp_runtime::{traits::BlockNumberProvider, SaturatedConversion};

const PDEX: u128 = 1000_000_000_000;

benchmarks! {
	set_migration_operational_status {
		let call = Call::<T>::set_migration_operational_status { status: true };
	}: { call.dispatch_bypass_filter(RawOrigin::Root.into())? }
	verify {
		assert!(<Operational<T>>::get());
	}

	set_relayer_status {
		let relayer : T::AccountId = account("relayer", 0, 0);
		let call = Call::<T>::set_relayer_status { relayer: relayer.clone(), status: true };
	}: { call.dispatch_bypass_filter(RawOrigin::Root.into())? }
	verify {
		assert!(<Relayers<T>>::get(relayer));
	}

	mint {
		let b in 1 .. 256;
		let relayer1: T::AccountId = account("relayer1",0,0);
		let relayer2: T::AccountId = account("relayer2",0,0);
		let relayer3: T::AccountId = account("relayer3",0,0);
		let beneficiary: T::AccountId  = whitelisted_caller();
		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let random_slice = [b as u8; 32];
		let mut eth_hash: T::Hash = T::Hash::default();
		eth_hash.as_mut().copy_from_slice(&random_slice);

			assert_ok!(PDEXMigration::<T>::set_migration_operational_status(RawOrigin::Root.into(),true));
			// Register relayers
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer1.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer2.clone(),true));
			assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(),relayer3.clone(),true));

			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer1).into(), beneficiary.clone(),amount,eth_hash));
			assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer2).into(), beneficiary.clone(),amount,eth_hash));

		let call = Call::<T>::mint { beneficiary, amount, eth_tx: eth_hash.clone().into() };
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(relayer3).into())? }
	verify {
		assert!(<EthTxns<T>>::contains_key(eth_hash));
	}

	unlock {
		let b in 1 .. 256;
		let relayer1 : T::AccountId = account("relayer1",0,0);
		let relayer2  : T::AccountId = account("relayer2",0,0);
		let relayer3 : T::AccountId = account("relayer3",0,0);
		let beneficiary : T::AccountId  = whitelisted_caller();

		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let random_slice = [b as u8; 32];
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
		let call = Call::<T>::unlock {};
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(beneficiary).into())? }

	remove_minted_tokens {
		let b in 1 .. 256;
		let relayer1: T::AccountId = account("relayer1",0,0);
		let relayer2  : T::AccountId = account("relayer2",0,0);
		let relayer3 : T::AccountId = account("relayer3",0,0);
		let beneficiary: T::AccountId  = whitelisted_caller();
		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let random_slice = [b as u8; 32];
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
		let call = Call::<T>::remove_minted_tokens { beneficiary };
	}: { call.dispatch_bypass_filter(RawOrigin::Root.into())? }
}

#[cfg(test)]
mod tests {
	use super::Pallet as PDM;
	use frame_benchmarking::impl_benchmark_test_suite;

	impl_benchmark_test_suite!(PDM, crate::mock::new_test_ext(), crate::mock::Test,);
}
