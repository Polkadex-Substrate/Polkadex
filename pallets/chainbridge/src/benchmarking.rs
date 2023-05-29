// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: GPL-3.0-only

//! Benchmarking setup for pallet-template

use frame_benchmarking::{benchmarks, whitelisted_caller};
use frame_system::RawOrigin;

#[allow(unused)]
use crate::Pallet as ChainBridge;

use super::*;

benchmarks! {
	set_threshold {
		let s in 0 .. 100;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), s)
	verify {
		assert_eq!(RelayerCount::<T>::get(), s);
	}

	impl_benchmark_test_suite!(ChainBridge, crate::mock::new_test_ext(), crate::mock::MockRuntime);
}
