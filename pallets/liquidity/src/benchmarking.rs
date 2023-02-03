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

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Benchmarking setup for liquidity pallet
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as liquidity;
use frame_benchmarking::{account, benchmarks};
use frame_support::{
    dispatch::UnfilteredDispatchable, traits::EnsureOrigin, BoundedBTreeMap, BoundedVec,
};
use frame_system::RawOrigin;
use parity_scale_codec::Decode;
use crate::pallet::{Call};

benchmarks! {
	register_account {
		let origin = T::GovernanceOrigin::successful_origin();
		let account_generation_key = 0_u32;
	}: {(origin, account_generation_key)}

	deposit_to_orderbook {
		let origin = T::GovernanceOrigin::successful_origin();
		let asset = AssetId::polakdex;
		let amount = 100_u128.saturated_into();
		let account_generation_key = 0_u32;
	}: {(origin, asset, amount, account_generation_key)}

	withdraw_from_orderbook {
		let origin = T::GovernanceOrigin::successful_origin();
		let asset = AssetId::polakdex;
		let amount = 100_u128.saturated_into();
		let do_force_withdraw = false;
		let account_generation_key = 0_u32;
	}: {(origin, asset, amount, do_force_withdraw account_generation_key)}


}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;
use polkadex_primitives::AssetId;
use sp_runtime::SaturatedConversion;

#[cfg(test)]
impl_benchmark_test_suite!(liquidity, crate::mock::new_test_ext(), crate::mock::Test);
