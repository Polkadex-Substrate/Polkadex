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


use super::*;
use frame_benchmarking::{benchmarks, whitelisted_caller, account};
use frame_system::{RawOrigin, Origin};
use sp_runtime::AccountId32;
use frame_support::assert_ok;
use crate::Pallet as OCEX;
use frame_system::EventRecord;
use crate::Event::MainAccountRegistered;
use sp_core::H256;
use polkadex_primitives::snapshot::{EnclaveSnapshot};
use polkadex_primitives::{WithdrawalLimit, AssetsLimit};
use frame_support::bounded_vec;
use codec::Decode;
// use crate::mock::Assets;

fn gen_signature<T: Config>() -> T::Signature{
	let sig_base: [u8; 64] = [194, 86, 40, 181, 200, 12, 205, 254, 172, 88, 86, 216, 236, 4, 116, 67, 185, 40, 6, 107, 15, 12, 77, 115, 8, 67, 3, 209, 139, 154, 95, 53, 178, 228, 234, 214, 42, 86, 92, 170, 142, 42, 50, 238, 76, 208, 55, 118, 34, 59, 62, 159, 91, 212, 25, 79, 180, 242, 100, 113, 51, 156, 163, 139];
	let sig_base_vec = sig_base.to_vec();
	let signature = T::Signature::decode(
		&mut sig_base_vec.as_ref(),
	).unwrap();
	return signature;
}

fn create_asset<T: Config>() {
	let caller: T::AccountId = account("caller",0,0);

	assert_ok!(T::OtherAssets::create(
		10_u128,
		caller.clone(),
		true,
		1_000_000_u32.into()
	));

	assert_ok!(
		T::OtherAssets::mint_into(
			10_u128,
			&caller,
			1_000_000_000_u32.into()
		)
	);
}
fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::Event = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

benchmarks! {
	register_main_account{
		let caller: T::AccountId = account("caller", 0, 0);
		let proxy: T::AccountId = account("proxy", 0, 0);
	}: _(RawOrigin::Signed(caller.clone()), proxy.clone())
	verify {
		let caller: T::AccountId = account("caller", 0, 0);
		let proxy: T::AccountId = account("proxy", 0, 0);
		assert_last_event::<T>(MainAccountRegistered{main:caller, proxy: proxy}.into());
	}

	add_proxy_account{
		let caller: T::AccountId = account("caller", 0, 0); 
		let main = account("main", 0, 0); 
		let proxy = account("proxy",0,0);
		assert_ok!(OCEX::<T>::register_main_account(RawOrigin::Signed(caller.clone()).into(), main));
	}: _(RawOrigin::Signed(caller), proxy)
	verify{
		let proxy = account("proxy",0,0);
		let caller: T::AccountId = account("caller", 0, 0); 
		assert_last_event::<T>(MainAccountRegistered{main: caller, proxy: proxy}.into());
	}

	close_trading_pair{
		let base: AssetId = AssetId::asset(10);
		let quote: AssetId = AssetId::asset(20);
		assert_ok!(
			OCEX::<T>::register_trading_pair(
				RawOrigin::Root.into(), 
				AssetId::asset(10), 
				AssetId::asset(20), 
				1_u32.into(),
				100_u32.into(), 
				1_u32.into(), 
				100_u32.into(),
				100_u32.into(),
				10_u32.into()
			)
		);
	}: _(RawOrigin::Root, base, quote) 
	verify{
		let trading_pair = OCEX::<T>::trading_pairs(base, quote).unwrap();
		assert_last_event::<T>(Event::ShutdownTradingPair{pair:trading_pair}.into());
	}

	open_trading_pair{
		let base: AssetId = AssetId::asset(10);
		let quote: AssetId = AssetId::asset(20); 
		assert_ok!(
			OCEX::<T>::register_trading_pair(
				RawOrigin::Root.into(), 
				AssetId::asset(10), 
				AssetId::asset(20), 
				1_u32.into(),
				100_u32.into(), 
				1_u32.into(), 
				100_u32.into(),
				100_u32.into(),
				10_u32.into()
			)
		);
	}: _(RawOrigin::Root, base, quote)
	verify{
		let trading_pair = OCEX::<T>::trading_pairs(base, quote).unwrap();
		assert_last_event::<T>(Event::OpenTradingPair{pair:trading_pair}.into());
	}

	register_trading_pair{
		let base: AssetId = AssetId::asset(10);
		let quote: AssetId = AssetId::asset(20);
		let min_trade_amount: u32 = 100_u32;
		let max_trade_amount: u32 = 1000_u32;
		let min_order_qty: u32 = 100_u32; 
		let max_order_qty: u32 = 1000_u32; 
		let max_spread: u32 = 100_u32; 
		let min_depth: u32 = 1_u32;
	}: _(RawOrigin::Root, base, quote, min_trade_amount.into(), max_trade_amount.into(), min_order_qty.into(), max_order_qty.into(), max_spread.into(), min_depth.into())
	verify{
		let trading_pair = OCEX::<T>::trading_pairs(base, quote).unwrap();
		assert_last_event::<T>(Event::TradingPairRegistered{base, quote}.into());
	} 
	
	deposit{
		let caller: T::AccountId = account("caller", 0, 0);
		let asset = AssetId::asset(10);
		let amount = 100000000_u32;
		create_asset::<T>();

	}: _(RawOrigin::Signed(caller.clone()), asset, amount.into())
	verify{
		let balance_amount: BalanceOf::<T> = amount.into();
		assert_last_event::<T>(Event::DepositSuccessful{user: caller, asset: asset, amount: balance_amount}.into());
	}

	collect_fees{
		let caller = account("caller",0,0);
		let snapshot_id: u32 = 1;
		let beneficiary = account("beneficiary",0,0);
	}: _(RawOrigin::Signed(caller), snapshot_id, beneficiary)
	verify{
		// TODO! this requires snapshot to be submiited 
	}

	shutdown{}:_(RawOrigin::Root)
	verify{
		// TODO! this requires an assertion from ingress messages 
	} 

	withdraw{
		let caller = account("caller",0,0);
		let snapshot_id: u32 = 1;
		let withdrawal_index: u32 = 2;
	}: _(RawOrigin::Signed(caller), snapshot_id, withdrawal_index)
	verify{
		// TODO! this requires a snapshot that contains an active withdrawal index
	}

	register_enclave{
		let caller: T::AccountId = account("caller",0,0);
		let ias_report: Vec<u8> = vec![];
	}: _(RawOrigin::Signed(caller.clone()), ias_report)
	verify{
		assert_last_event::<T>(Event::EnclaveRegistered(caller).into());
		// TODO
	}
	submit_snapshot{
		let caller = account("caller",0,0);
		let mmr_root: H256 = H256::from_slice(&[210, 56, 200, 34, 238, 216, 179, 3, 145, 126, 70, 52, 246, 40, 114, 190, 67, 101, 64, 12, 96, 36, 91, 129, 237, 83, 237, 98, 171, 246, 205, 98]);
		let mut snapshot = EnclaveSnapshot::<T::AccountId, BalanceOf::<T>, WithdrawalLimit, AssetsLimit>{
			snapshot_number: 0,
    		merkle_root: mmr_root,
			withdrawals: bounded_vec![],
    		fees: bounded_vec![],
		};
		let signature = gen_signature::<T>();
	}: _(RawOrigin::Signed(caller), snapshot, signature)
	verify{
		// TODO! Query some events over here
	} 



	impl_benchmark_test_suite!(Pallet, crate::mock::new_test_ext(), crate::mock::Test)
}
