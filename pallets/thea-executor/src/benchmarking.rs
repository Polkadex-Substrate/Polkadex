// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::Pallet as TheaExecutor;
use frame_benchmarking::v1::{account, benchmarks};
use frame_support::traits::{
	fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
	fungibles::{Create, Mutate},
	Get, OnInitialize,
};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use parity_scale_codec::Decode;
use polkadex_primitives::UNIT_BALANCE;
use sp_runtime::{traits::AccountIdConversion, SaturatedConversion};
use sp_std::{boxed::Box, vec};
use thea_primitives::types::{AssetMetadata, Withdraw};
use xcm::VersionedMultiLocation;

benchmarks! {
	set_withdrawal_fee {
		let r in 1 .. 1000;
		let network_id = r as u8;
		let fee = 1_000_000_000_000;
	}: _(RawOrigin::Root, network_id, fee)
	verify {
		assert_eq!(<WithdrawalFees<T>>::get(network_id), Some(fee));
	}

	update_asset_metadata {
		let r in 1 .. 1000;
		let asset_id = r as u128;
		let decimal: u8 = 8;
	}: _(RawOrigin::Root, asset_id, decimal)
	verify {
		let metadata = AssetMetadata::new(decimal).unwrap();
		assert_eq!(<Metadata<T>>::get(asset_id), Some(metadata));
	}

	withdraw {
		let r in 1 .. 1000;
		let asset_id: <T as pallet::Config>::AssetId = 100u128.into();
		let admin = account::<T::AccountId>("admin", 1, r);
		let network_id = 1;
		<T as pallet::Config>::Currency::mint_into(&admin, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		<T as pallet::Config>::Assets::create(asset_id.into(), admin.clone(), true, 1u128.saturated_into()).unwrap();
		let account = account::<T::AccountId>("alice", 1, r);
		<T as pallet::Config>::Assets::mint_into(asset_id.into(), &account, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		<T as pallet::Config>::Currency::mint_into(&account, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let pallet_acc = T::TheaPalletId::get().into_account_truncating();
		<T as pallet::Config>::Currency::mint_into(&pallet_acc, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let metadata = AssetMetadata::new(3).unwrap();
		<Metadata<T>>::insert(100, metadata);
		<WithdrawalFees<T>>::insert(network_id, 10);
		let benificary = vec![1;32];
	}: _(RawOrigin::Signed(account.clone()), 100, 1_000, benificary, true, network_id, false)
	verify {
		let ready_withdrawal = <ReadyWithdrawals<T>>::get(<frame_system::Pallet<T>>::block_number(), network_id);
		assert_eq!(ready_withdrawal.len(), 1);
	}

	parachain_withdraw {
		let r in 1 .. 1000;
		let asset_id: <T as pallet::Config>::AssetId = 100u128.into();
		let admin = account::<T::AccountId>("admin", 1, r);
		let network_id = 1;
		<T as pallet::Config>::Assets::create(asset_id.into(), admin, true, 1u128.saturated_into()).unwrap();
		let pallet_acc = T::TheaPalletId::get().into_account_truncating();
		<T as pallet::Config>::Currency::mint_into(&pallet_acc, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let account = account::<T::AccountId>("alice", 1, r);
		<T as pallet::Config>::Assets::mint_into(asset_id.into(), &account, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		<T as pallet::Config>::Currency::mint_into(&account, 100_000_000_000_000u128.saturated_into()).unwrap();
		let metadata = AssetMetadata::new(10).unwrap();
		<Metadata<T>>::insert(100, metadata);
		<WithdrawalFees<T>>::insert(network_id, 1_000);
		let multilocation = MultiLocation { parents: 1, interior: Junctions::Here };
		let benificary = VersionedMultiLocation::V3(multilocation);
	}: _(RawOrigin::Signed(account.clone()), 100, 1_000_000_000_000, Box::new(benificary), true, false)
	verify {
		let ready_withdrawal = <ReadyWithdrawals<T>>::get(<frame_system::Pallet<T>>::block_number(), network_id);
		assert_eq!(ready_withdrawal.len(), 1);
	}

	ethereum_withdraw {
		let r in 1 .. 1000;
		let asset_id: <T as pallet::Config>::AssetId = 100u128.into();
		let admin = account::<T::AccountId>("admin", 1, r);
		let network_id = 2;
		<T as pallet::Config>::Assets::create(asset_id.into(), admin, true, 1u128.saturated_into()).unwrap();
		let pallet_acc = T::TheaPalletId::get().into_account_truncating();
		<T as pallet::Config>::Currency::mint_into(&pallet_acc, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let account = account::<T::AccountId>("alice", 1, r);
		<T as pallet::Config>::Assets::mint_into(asset_id.into(), &account, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		<T as pallet::Config>::Currency::mint_into(&account, 100_000_000_000_000u128.saturated_into()).unwrap();
		let metadata = AssetMetadata::new(10).unwrap();
		<Metadata<T>>::insert(100, metadata);
		<WithdrawalFees<T>>::insert(network_id, 1_000);
		let beneficiary: sp_core::H160 = sp_core::H160::default();
	}: _(RawOrigin::Signed(account.clone()), 100, 1_000_000_000_000, beneficiary, true, false)
	verify {
		let ready_withdrawal = <ReadyWithdrawals<T>>::get(<frame_system::Pallet<T>>::block_number(), network_id);
		assert_eq!(ready_withdrawal.len(), 1);
	}

	on_initialize {
		// Insert Withdrawals in ReadyWithdrawals
		let withdrawal = Withdraw {
			id: vec![],
			asset_id: 100,
			amount: 1_000_000_000_000,
			destination: vec![],
			is_blocked: false,
			extra: vec![],
		};
		let withdrawal_vec = vec![withdrawal; 30];
		let block_no: u32 = 10;
		let networks = vec![1,2,3,4,5,6,7,8,9,10,11,12,13,14,15];
		let block_no: BlockNumberFor<T> = block_no.into();
		for network_id in networks {
			<ReadyWithdrawals<T>>::insert(block_no, network_id, withdrawal_vec.clone());
		}
	}: {
		TheaExecutor::<T>::on_initialize(block_no);
	}

	burn_native_tokens{
		let account: T::AccountId = T::AccountId::decode(&mut &[0u8; 32][..]).unwrap();
		<T as pallet::Config>::Currency::mint_into(&account, (100000*UNIT_BALANCE).saturated_into()).unwrap();
	}: _(RawOrigin::Root, account.clone(), UNIT_BALANCE)
	verify {
		assert_eq!(<T as pallet::Config>::Currency::balance(&account), (99999 * UNIT_BALANCE).saturated_into());
	}
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;
use xcm::latest::{Junctions, MultiLocation};

#[cfg(test)]
impl_benchmark_test_suite!(TheaExecutor, crate::mock::new_test_ext(), crate::mock::Test);
