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

//! Benchmarking for pallet-example-basic.

#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::{benchmarks, allowlisted_caller};
use frame_system::RawOrigin;

benchmarks! {
	set_dummy_benchmark {
		let b in 1 .. 1000;
	}: set_dummy(RawOrigin::Root, b.into())
	verify {
		assert_eq!(Pallet::<T>::dummy(), Some(b.into()))
	}
	accumulate_dummy {
		let b in 1 .. 1000;
		let caller: T::AccountId = allowlisted_caller();
	}: _(RawOrigin::Signed(caller), b.into())

	sort_vector {
		let x in 0 .. 10000;
		let mut m = Vec::<u32>::new();
		for i in (0..x).rev() {
			m.push(i);
		}
	}: {
		m.sort();
	}


	impl_benchmark_test_suite!(Pallet, crate::tests::new_test_ext(), crate::tests::Test)
}
