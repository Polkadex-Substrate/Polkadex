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

//! Benchmarking setup for pallet-ocex
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::pallet::Call;
use frame_benchmarking::benchmarks;
use frame_support::{dispatch::UnfilteredDispatchable, traits::EnsureOrigin};
use parity_scale_codec::Decode;
use sp_std::vec;

benchmarks! {
	allow_list_token_for_fees {
		let b in 0 .. 255;
		let origin = T::GovernanceOrigin::successful_origin();
		let asset = AssetIdOf::<T>::decode(&mut [b as u8; 32].as_ref()).unwrap();
		let call = Call::<T>::allow_list_token_for_fees{ asset: asset.clone() };
	}: { call.dispatch_bypass_filter(origin.clone())? }
	verify {
		assert!(<AllowedAssets<T>>::get().contains(&asset));
	}

	block_token_for_fees {
		let b in 0 .. 255;
		let origin = T::GovernanceOrigin::successful_origin();
		let asset = AssetIdOf::<T>::decode(&mut [b as u8; 32].as_ref()).unwrap();
		let allowed = vec!(asset.clone());
		<AllowedAssets<T>>::set(allowed);
		let call = Call::<T>::block_token_for_fees{ asset: asset.clone() };
	}: { call.dispatch_bypass_filter(origin.clone())? }
	verify {
		assert!(<AllowedAssets<T>>::get().is_empty());
	}
}

#[cfg(test)]
use crate::Pallet as ATP;

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(ATP, crate::mock::new_test_ext(), crate::mock::Test);
