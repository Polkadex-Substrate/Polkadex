// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Benchmarking setup for pallet-pdex-migration
use frame_benchmarking::{account, benchmarks, whitelisted_caller};
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable, traits::Get};
use frame_system::RawOrigin;
use parity_scale_codec::Decode;
use sp_runtime::{traits::BlockNumberProvider, SaturatedConversion};

use crate::pallet::{Call, Config, Pallet as PDEXMigration, Pallet, *};

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
		let b in 1 .. 254;
		let relayer1: T::AccountId = account("relayer1", 0, 0);
		let relayer2: T::AccountId = account("relayer2", 0, 0);
		let relayer3: T::AccountId = account("relayer3", 0, 0);
		let beneficiary: T::AccountId  = whitelisted_caller();
		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let eth_hash: T::Hash = T::Hash::decode(&mut [b as u8; 32].as_ref()).unwrap();

		assert_ok!(PDEXMigration::<T>::set_migration_operational_status(RawOrigin::Root.into(),true));
		// Register relayers
		assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(), relayer1.clone(), true));
		assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(), relayer2.clone(), true));
		assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(), relayer3.clone(), true));
		assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer1).into(), beneficiary.clone(), amount, eth_hash));
		assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer2).into(), beneficiary.clone(), amount, eth_hash));

		let call = Call::<T>::mint { beneficiary, amount, eth_tx: eth_hash.clone().into() };
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(relayer3).into())? }
	verify {
		assert!(<EthTxns<T>>::contains_key(eth_hash));
	}

	unlock {
		let b in 1 .. 254;
		let relayer1 : T::AccountId = account("relayer1",0,0);
		let relayer2  : T::AccountId = account("relayer2",0,0);
		let relayer3 : T::AccountId = account("relayer3",0,0);
		let beneficiary : T::AccountId  = whitelisted_caller();

		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let eth_hash: T::Hash = T::Hash::decode(&mut [b as u8; 32].as_ref()).unwrap();

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
		let b in 1 .. 254;
		let relayer1: T::AccountId = account("relayer1",0,0);
		let relayer2  : T::AccountId = account("relayer2",0,0);
		let relayer3 : T::AccountId = account("relayer3",0,0);
		let beneficiary: T::AccountId  = whitelisted_caller();
		let amount: T::Balance = 100u128.saturating_mul(PDEX).saturated_into();
		let eth_hash: T::Hash = T::Hash::decode(&mut [b as u8; 32].as_ref()).unwrap();

		assert_ok!(PDEXMigration::<T>::set_migration_operational_status(RawOrigin::Root.into(), true));
		// Register relayers
		assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(), relayer1.clone(), true));
		assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(), relayer2.clone(), true));
		assert_ok!(PDEXMigration::<T>::set_relayer_status(RawOrigin::Root.into(), relayer3.clone(), true));
		assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer1).into(), beneficiary.clone(), amount, eth_hash));
		assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer2).into(), beneficiary.clone(), amount, eth_hash));
		assert_ok!(PDEXMigration::<T>::mint(RawOrigin::Signed(relayer3).into(), beneficiary.clone(), amount, eth_hash));
		let call = Call::<T>::remove_minted_tokens { beneficiary };
	}: { call.dispatch_bypass_filter(RawOrigin::Root.into())? }
}

#[cfg(test)]
mod tests {
	use frame_benchmarking::impl_benchmark_test_suite;

	use super::Pallet as PDM;

	impl_benchmark_test_suite!(PDM, crate::mock::new_test_ext(), crate::mock::Test,);
}
