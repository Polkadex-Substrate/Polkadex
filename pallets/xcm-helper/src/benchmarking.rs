// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

use super::*;

#[allow(unused_imports)]
use crate::Pallet as XcmHelper;
use frame_benchmarking::{account, benchmarks};
use frame_support::{
	sp_runtime::SaturatedConversion,
	traits::fungible::{Inspect as NativeInspect, Mutate as NativeMutate},
};
use frame_system::RawOrigin;

use sp_core::Get;
use sp_runtime::traits::AccountIdConversion;
use xcm::latest::{AssetId, Junction, Junctions, MultiLocation};
const SEED: u32 = 0;

benchmarks! {
	whitelist_token {
		let b in 1 .. 1000;
		let token = b as u128;
		let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(b)));
		let token: AssetId = AssetId::Concrete(asset_location);
	}: _(RawOrigin::Root, token)
	verify {
		let token = XcmHelper::<T>::generate_asset_id_for_parachain(token);
		let whitelisted_tokens = <WhitelistedTokens<T>>::get();
		assert!(whitelisted_tokens.contains(&token));
	}

	remove_whitelisted_token {
		let b in 1 .. 1000;
		let token = b as u128;
		let asset_location = MultiLocation::new(1, Junctions::X1(Junction::Parachain(b)));
		let token: AssetId = AssetId::Concrete(asset_location);
		let token_id = XcmHelper::<T>::generate_asset_id_for_parachain(token);
		let mut whitelisted_tokens = <WhitelistedTokens<T>>::get();
		whitelisted_tokens.push(token_id);
		<WhitelistedTokens<T>>::put(whitelisted_tokens);
	}: _(RawOrigin::Root, token)
	verify {
		let whitelisted_tokens = <WhitelistedTokens<T>>::get();
		assert!(!whitelisted_tokens.contains(&token_id));
	}

	transfer_fee {
		let b in 1 .. 1000;
		let pallet_account: T::AccountId = T::AssetHandlerPalletId::get().into_account_truncating();
		let asset = T::NativeAssetId::get();
		T::Currency::mint_into(&pallet_account, 2_000_000_000_000_000u128.saturated_into()).unwrap();
		let recipeint: T::AccountId = account("mem1", b, SEED);
	}: _(RawOrigin::Root, recipeint.clone())
	verify {
		assert_eq!(T::Currency::balance(&recipeint), 1999000000000000u128.saturated_into());
	}
	// TODO: We need to adapt this benchmark to work in runtime context
	// on_initialize {
	// 	let x: T::BlockNumber = 1u64.saturated_into();
	// 	let pallet_account: T::AccountId = T::AssetHandlerPalletId::get().into_account_truncating();
	// 	let asset = T::NativeAssetId::get();
	// 	let asset_id = AssetId::Concrete(MultiLocation::new(1, Junctions::X1(Junction::Parachain(1000))));
	// 	let asset_id = XcmHelper::<T>::generate_asset_id_for_parachain(asset_id);
	// 	T::AssetManager::mint_into(
	// 		asset,
	// 		&pallet_account,
	// 		2_000_000_000_000_000u128.saturated_into()
	// 	).unwrap();
	// 	T::AssetManager::create(
	// 			asset_id,
	// 			T::AssetHandlerPalletId::get().into_account_truncating(),
	// 			true,
	// 			1u128,
	// 		)?;
	// 	<ParachainAssets<T>>::insert(asset_id,
	// 		AssetId::Concrete(MultiLocation::new(1, Junctions::X1(Junction::Parachain(1000)))));
	// 	let withdrawals: Vec<Withdraw> = vec![Withdraw {
	// 		id: Vec::new(),
	// asset_id: asset_id,
	// amount: UNIT_BALANCE * 10,
	// destination: VersionedMultiLocation::V1(MultiLocation::new(1,
	// 														 Junctions::X2(
	// 															 Junction::Parachain(1000),
	// 															 Junction::PalletInstance(1)
	// 														 )
	// )).encode(),
	// is_blocked: false,
	// extra: Vec::new(),
	// 	};100];
	// 	<PendingWithdrawals<T>>::insert(x,withdrawals);
	// }: {
	// 	Pallet::<T>::on_initialize(x);
	// } verify {
	// 	let withdrawals = <PendingWithdrawals<T>>::get(x);
	// 	let failed_withdrawals = <FailedWithdrawals<T>>::get(x);
	// 	assert!(failed_withdrawals.is_empty());
	// 	assert!(withdrawals.is_empty())
	// }
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(XcmHelper, crate::mock::new_test_ext(), crate::mock::Test);
