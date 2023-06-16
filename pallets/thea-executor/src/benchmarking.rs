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
use parity_scale_codec::Decode;
use sp_runtime::traits::AccountIdConversion;
use sp_std::{boxed::Box, vec, vec::Vec};

use frame_benchmarking::v1::{account, benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::{
	ensure,
	traits::{
		fungible::Mutate as NativeMutate,
		fungibles::{Create, Inspect, Mutate},
		EnsureOrigin, Get,
	},
};
use frame_system::RawOrigin;
use sp_runtime::{traits::Bounded, SaturatedConversion};
use thea_primitives::types::{AssetMetadata, Deposit};
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

	claim_deposit {
		let r in 1 .. 1000;
		let account = account::<T::AccountId>("alice", 1, r);
		let asset_id: T::AssetId = 100u128.into();
		let deposits = create_deposit::<T>(account.clone());
		let metadata = AssetMetadata::new(10).unwrap();
		<Metadata<T>>::insert(100, metadata);
		T::Currency::mint_into(&account, 100_000_000_000_000u128.saturated_into()).unwrap();
		<ApprovedDeposits<T>>::insert(account.clone(), deposits);
	}: _(RawOrigin::Signed(account.clone()), 10)
	verify {
		let current_balance = T::Assets::balance(asset_id.into(), &account);
		assert_eq!(current_balance, 1_000_000_000_000_000u128.saturated_into()); //TODO: Verify this value
	}

	withdraw {
		let r in 1 .. 1000;
		let asset_id: T::AssetId = 100u128.into();
		let admin = account::<T::AccountId>("admin", 1, r);
		let network_id = 1;
		T::Currency::mint_into(&admin, 100_000_000_000_000_000_000u128.saturated_into());
		T::Assets::create(asset_id.into(), admin.clone(), true, 1u128.saturated_into()).unwrap();
		let account = account::<T::AccountId>("alice", 1, r);
		T::Assets::mint_into(asset_id.into(), &account, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		T::Currency::mint_into(&account, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let pallet_acc = T::TheaPalletId::get().into_account_truncating();
		T::Currency::mint_into(&pallet_acc, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let metadata = AssetMetadata::new(3).unwrap();
		<Metadata<T>>::insert(100, metadata);
		<WithdrawalFees<T>>::insert(network_id, 10);
		let benificary = vec![1;32];
	}: _(RawOrigin::Signed(account.clone()), 100, 1_000, benificary, true, network_id)
	verify {
		let ready_withdrawal = <ReadyWithdrawals<T>>::get(<frame_system::Pallet<T>>::block_number(), network_id);
		assert_eq!(ready_withdrawal.len(), 1);
	}

	parachain_withdraw {
		let r in 1 .. 1000;
		let asset_id: T::AssetId = 100u128.into();
		let admin = account::<T::AccountId>("admin", 1, r);
		let network_id = 1;
		T::Assets::create(asset_id.into(), admin, true, 1u128.saturated_into());
		let pallet_acc = T::TheaPalletId::get().into_account_truncating();
		T::Currency::mint_into(&pallet_acc, 100_000_000_000_000_000_000u128.saturated_into()).unwrap();
		let account = account::<T::AccountId>("alice", 1, r);
		T::Assets::mint_into(asset_id.into(), &account, 100_000_000_000_000_000_000u128.saturated_into());
		T::Currency::mint_into(&account, 100_000_000_000_000u128.saturated_into());
		let metadata = AssetMetadata::new(10).unwrap();
		<Metadata<T>>::insert(100, metadata);
		<WithdrawalFees<T>>::insert(network_id, 1_000);
		let multilocation = MultiLocation { parents: 1, interior: Junctions::Here };
		let benificary = VersionedMultiLocation::V3(multilocation);
	}: _(RawOrigin::Signed(account.clone()), 100, 1_000_000_000_000, Box::new(benificary), true)
	verify {
		let ready_withdrawal = <ReadyWithdrawals<T>>::get(<frame_system::Pallet<T>>::block_number(), network_id);
		assert_eq!(ready_withdrawal.len(), 1);
	}
}

fn create_deposit<T: Config>(recipient: T::AccountId) -> Vec<Deposit<T::AccountId>> {
	let mut pending_deposits = vec![];
	let asset_id = 100;
	for i in 1..20 {
		let deposit: Deposit<T::AccountId> = Deposit {
			id: vec![],
			recipient: recipient.clone(),
			asset_id,
			amount: 1_000_000_000_000,
			extra: vec![],
		};
		pending_deposits.push(deposit);
	}
	pending_deposits
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;
use xcm::latest::{Junctions, MultiLocation};

#[cfg(test)]
impl_benchmark_test_suite!(TheaExecutor, crate::mock::new_test_ext(), crate::mock::Test);
