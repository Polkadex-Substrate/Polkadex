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
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::RawOrigin;
use sp_runtime::AccountId32;
use crate::mock::Test;

benchmarks! {
	register_main_account{
		let caller = account("caller", 0, 0);
		let proxy = account("proxy", 0, 0);
	}: _(RawOrigin::Signed(caller), proxy)
	verify {
		// assert_eq!(Pallet::<T>::dummy(), Some(b.into()))
	}

	add_proxy_account{
		let caller = account("caller", 0, 0); 
		let proxy = account("proxy",0,0);
	}: _(RawOrigin::Signed(caller), proxy)
	verify{
		// Query events or storage 
	}

	close_trading_pair{
		let base: AssetId = AssetId::asset(10);
		let quote: AssetId = AssetId::asset(20);
	}: _(RawOrigin::Root, base, quote) 
	verify{
		// Query events or storage 
	}

	open_trading_pair{
		let base: AssetId = AssetId::asset(10);
		let quote: AssetId = AssetId::asset(20); 
	}: _(RawOrigin::Root, base, quote)
	verify{
		// Query event or storage
	}

	/* register_trading_pair{
		let base: AssetId = AssetId::asset(10);
		let quote: AssetId = AssetId::asset(20);
		let min_trade_amount: BalanceOf::<Test> = 100;
		let max_trade_amount: BalanceOf::<Test> = 1000;
		let min_order_qty: BalanceOf::<Test> = 100; 
		let max_order_qty: BalanceOf::<Test> = 1000; 
		let max_spread: BalanceOf::<Test> = 100; 
		let min_depth: BalanceOf::<Test> = 1;
	}: _(RawOrigin::Root, base, quote, min_trade_amount.into(), max_trade_amount.into(), min_order_qty.into(), max_order_qty.into(), max_spread.into(), min_depth.into())
	verify{
		// Query events or storage 
	} */
	/* accumulate_dummy {
		let b in 1 .. 1000;
		let caller: T::AccountId = whitelisted_caller();
	}: _(RawOrigin::Signed(caller), b.into())

	sort_vector {
		let x in 0 .. 10000;
		let mut m = Vec::<u32>::new();
		for i in (0..x).rev() {
			m.push(i);
		}
	}: {
		m.sort();
	} */


	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test)
}
