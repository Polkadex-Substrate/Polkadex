<<<<<<< .merge_file_w6EY0n
=======
// This file is part of Polkadex.

>>>>>>> .merge_file_p0ZnrF
// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
<<<<<<< .merge_file_w6EY0n
// GNU General Public License for more details.unwrap().

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Benchmarking setup for pallet-ocex
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Ocex;
use codec::Decode;
use frame_benchmarking::{account, benchmarks};
use frame_support::{
	dispatch::UnfilteredDispatchable, traits::EnsureOrigin, BoundedBTreeMap, BoundedVec,
};
use frame_system::RawOrigin;
use polkadex_primitives::{
	ocex::TradingPairConfig,
	snapshot::{EnclaveSnapshot, Fees},
	withdrawal::Withdrawal,
	ProxyLimit, WithdrawalLimit, UNIT_BALANCE,
};
use rust_decimal::{prelude::*, Decimal};
use sp_runtime::BoundedBTreeSet;
use test_utils::ias::ias::*;

// Check if last event generated by pallet is the one we're expecting
fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn convert_to_balance<T: Config>(dec: Decimal) -> BalanceOf<T> {
	BalanceOf::<T>::decode(
		&mut &dec.saturating_mul(UNIT_BALANCE.into()).to_u128().unwrap().to_le_bytes()[..],
	)
	.unwrap()
}

fn tpc(base_asset: AssetId, quote_asset: AssetId) -> TradingPairConfig {
	TradingPairConfig {
		base_asset,
		quote_asset,
		min_price: Decimal::from_f32(0.0001).unwrap(),
		max_price: Decimal::from_f32(100000.0).unwrap(),
		price_tick_size: Decimal::from_f32(0.000001).unwrap(),
		min_qty: Decimal::from_f64(0.001).unwrap(),
		max_qty: Decimal::from_f32(10000.0).unwrap(),
		qty_step_size: Decimal::from_f64(0.001).unwrap(),
		operational_status: true,
		base_asset_precision: 1,
		quote_asset_precision: 1,
	}
}

// All benchmarks names match extrinsic names so we call them with `_()`
benchmarks! {
	// pass
	register_main_account {
		let b in 0 .. 50_000;
		let origin = T::EnclaveOrigin::successful_origin();
		let account = T::EnclaveOrigin::successful_origin();
		let main: T::AccountId = match unsafe { origin.clone().into().unwrap_unchecked() } {
			RawOrigin::Signed(account) => account.into(),
			_ => panic!("wrong RawOrigin returned")
		};
		let proxy: T::AccountId = match unsafe { account.into().unwrap_unchecked() } {
			RawOrigin::Signed(account) => account.into(),
			_ => panic!("wrong RawOrigin returned")
		};
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::register_main_account { proxy: proxy.clone() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::MainAccountRegistered {
			main,
			proxy
		}.into());
	}

	// pass
	add_proxy_account {
		let x in 0 .. 255; // should not overflow u8
		let origin = T::EnclaveOrigin::successful_origin();
		let main: T::AccountId = match unsafe { origin.clone().into().unwrap_unchecked() } {
			RawOrigin::Signed(account) => account.into(),
			_ => panic!("wrong RawOrigin returned")
		};
		let proxy = T::AccountId::decode(&mut &[x as u8; 32].to_vec()[..]).unwrap();
		<ExchangeState<T>>::put(true);
		Ocex::<T>::register_main_account(origin.clone(), main.clone())?;
		let call = Call::<T>::add_proxy_account { proxy: proxy.clone() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::MainAccountRegistered {
			main,
			proxy
		}.into());
	}

	// pass
	close_trading_pair {
		let x in 1 .. 50_000;
		let origin = T::GovernanceOrigin::successful_origin();
		let base = AssetId::asset(x.into());
		let quote = AssetId::asset((x + 1).into());
		let config = tpc(base, quote);
		<TradingPairs<T>>::insert(base, quote, config);
		let pair = <TradingPairs<T>>::get(base, quote).unwrap();
		let expected_pair = TradingPairConfig {
			operational_status: false,
			..pair
		};
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::close_trading_pair { base, quote };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::ShutdownTradingPair {
			pair: expected_pair
		}.into());
	}

	// pass
	open_trading_pair {
		let x in 0 .. 100_000;
		let origin = T::GovernanceOrigin::successful_origin();
		let base = AssetId::asset(x.into());
		let quote = AssetId::asset((x + 1).into());
		let config = tpc(base, quote);
		<TradingPairs<T>>::insert(base, quote, config.clone());
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::open_trading_pair { base, quote };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::OpenTradingPair {
			pair: config,
		}.into());
	}

	// pass
	register_trading_pair {
		let x in 0 .. 100_000;
		let origin = T::GovernanceOrigin::successful_origin();
		let base = AssetId::asset(x.into());
		let quote = AssetId::asset((x + 1).into());
		let TradingPairConfig{
			base_asset,
			quote_asset,
			min_price,
			max_price,
			min_qty,
			max_qty,
			operational_status,
			price_tick_size,
			qty_step_size,
			base_asset_precision,
			quote_asset_precision,
			} = tpc(base, quote);
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::register_trading_pair {
			base,
			quote,
			min_order_price: convert_to_balance::<T>(min_price),
			max_order_price: convert_to_balance::<T>(max_price),
			min_order_qty: convert_to_balance::<T>(min_qty),
			max_order_qty: convert_to_balance::<T>(max_qty),
			price_tick_size: convert_to_balance::<T>(price_tick_size),
			qty_step_size: convert_to_balance::<T>(qty_step_size)
		};
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::TradingPairRegistered {
			base,
			quote
		}.into());
	}

	// pass
	update_trading_pair {
		let x in 0 .. 100_000;
		let origin = T::GovernanceOrigin::successful_origin();
		let base = AssetId::asset(x.into());
		let quote = AssetId::asset((x + 1).into());
		let mut tp = tpc(base, quote);
		let TradingPairConfig{
			base_asset,
			quote_asset,
			min_price,
			max_price,
			min_qty,
			max_qty,
			operational_status,
			price_tick_size,
			qty_step_size,
			base_asset_precision,
			quote_asset_precision,
			} = tp.clone();
		let governance = T::GovernanceOrigin::successful_origin();
		Ocex::<T>::set_exchange_state(governance.clone(), true)?;
		tp.operational_status = false;
		<TradingPairs<T>>::insert(base_asset, quote_asset, tp);
		let call = Call::<T>::update_trading_pair {
			base,
			quote,
			min_order_price: convert_to_balance::<T>(min_price),
			max_order_price: convert_to_balance::<T>(max_price),
			min_order_qty: convert_to_balance::<T>(min_qty),
			max_order_qty: convert_to_balance::<T>(max_qty),
			price_tick_size: convert_to_balance::<T>(price_tick_size),
			qty_step_size: convert_to_balance::<T>(qty_step_size)
		};
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::TradingPairUpdated {
			base,
			quote
		}.into());
	}

	// pass
	deposit {
		let x in 1 .. 255; // should not overflow u8
		let user = account::<T::AccountId>("user", x, 0);
		let asset = AssetId::asset(x.into());
		let amount  = BalanceOf::<T>::decode(&mut &(x as u128).saturating_mul(10u128).to_le_bytes()[..]).unwrap();
		let governance = T::GovernanceOrigin::successful_origin();
		Ocex::<T>::set_exchange_state(governance.clone(), true)?;
		Ocex::<T>::allowlist_token(governance.clone(), asset.clone())?;
		use frame_support::traits::fungibles::Create;
		T::OtherAssets::create(
			x as u128,
			Ocex::<T>::get_pallet_account(),
			true,
			BalanceOf::<T>::one().unique_saturated_into())?;
		T::OtherAssets::mint_into(
			x as u128,
			&user.clone(),
			BalanceOf::<T>::decode(&mut &(u128::MAX).to_le_bytes()[..]).unwrap()
		)?;
		let proxy = account::<T::AccountId>("proxy", x, 0);
		Ocex::<T>::register_main_account(RawOrigin::Signed(user.clone()).into(), proxy)?;
		let call = Call::<T>::deposit { asset, amount };
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(user.clone()).into())? }
	verify {
		assert_last_event::<T>(Event::DepositSuccessful {
			user,
			asset,
			amount
		}.into());
	}

	// pass
	remove_proxy_account {
		let x in 1 .. 255; // should not overflow u8
		let main = account::<T::AccountId>("main", 0, 0);
		let proxy = T::AccountId::decode(&mut &[x as u8 ; 32].to_vec()[..]).unwrap();
		let governance = T::GovernanceOrigin::successful_origin();
		Ocex::<T>::set_exchange_state(governance.clone(), true)?;
		let signed = RawOrigin::Signed(main.clone());
		Ocex::<T>::register_main_account(signed.clone().into(), proxy.clone())?;
		// worst case scenario
		for i in 2 .. ProxyLimit::get() {
			let new_proxy = account::<T::AccountId>("proxy", i, 0);
			Ocex::<T>::add_proxy_account(signed.clone().into(), new_proxy)?;
		}
		let call = Call::<T>::remove_proxy_account { proxy: proxy.clone() };
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(main.clone()).into())? }
	verify {
		assert_last_event::<T>(Event::ProxyRemoved {
			main,
			proxy
		}.into());
	}

	// pass
	submit_snapshot {
		let origin = T::AccountId::decode(&mut &[6, 196, 28, 36, 60, 116, 41, 76, 197, 21, 40, 124, 17, 142, 128, 189, 115, 168, 219, 199, 151, 158, 208, 8, 177, 131, 105, 116, 42, 17, 129, 26][..]).unwrap();
		let snapshot = EnclaveSnapshot::decode(&mut &[1, 0, 0, 0, 50, 157, 46, 78, 212, 64, 1, 64, 121, 45, 35, 138, 120, 29, 202, 62, 154, 100, 140, 141, 191, 125, 221, 151, 154, 28, 82, 226, 137, 175, 36, 134, 4, 6, 196, 28, 36, 60, 116, 41, 76, 197, 21, 40, 124, 17, 142, 128, 189, 115, 168, 219, 199, 151, 158, 208, 8, 177, 131, 105, 116, 42, 17, 129, 26, 4, 6, 196, 28, 36, 60, 116, 41, 76, 197, 21, 40, 124, 17, 142, 128, 189, 115, 168, 219, 199, 151, 158, 208, 8, 177, 131, 105, 116, 42, 17, 129, 26, 0, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0][..]).unwrap();
		let signature: T::Signature = T::Signature::decode(&mut &[1, 184, 164, 69, 4, 236, 201, 207, 230, 19, 226, 51, 221, 175, 219, 188, 170, 247, 233, 188, 190, 176, 110, 201, 221, 1, 188, 190, 185, 107, 60, 138, 107, 127, 215, 181, 225, 118, 46, 13, 38, 102, 133, 69, 170, 169, 80, 114, 36, 202, 18, 13, 140, 135, 98, 90, 206, 14, 140, 61, 39, 168, 151, 191, 130][..]).unwrap();
		<RegisteredEnclaves<T>>::insert(&origin, T::Moment::default());
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::submit_snapshot { snapshot, signature };
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(origin).into())? }
	verify {
		assert!(<Snapshots<T>>::contains_key(1));
	}

	// pass
	insert_enclave {
		let x in 0 .. 255; // should not overflow u8
		let origin = T::GovernanceOrigin::successful_origin();
		let enclave = T::AccountId::decode(&mut &[x as u8; 32][..]).unwrap();
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::insert_enclave { enclave: enclave.clone() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert!(<RegisteredEnclaves<T>>::contains_key(enclave));
	}

	// pass
	collect_fees {
		let x in 0 .. 255; // should not overflow u8
		let origin = T::GovernanceOrigin::successful_origin();
		let beneficiary = T::AccountId::decode(&mut &[x as u8; 32][..]).unwrap();
		let fees: Fees = Fees { asset: AssetId::polkadex, amount: Decimal::new(100, 1) };
		let snapshot =
			EnclaveSnapshot::<_, _, _, _> {
				snapshot_number: x.into(),
				snapshot_hash: Default::default(),
				withdrawals: Default::default(),
				fees: BoundedVec::try_from(vec!(fees)).unwrap(),
			};
		<ExchangeState<T>>::put(true);
		<Snapshots<T>>::insert(x, snapshot);
		let call = Call::<T>::collect_fees { snapshot_id: x, beneficiary: beneficiary.clone() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::FeesClaims{ beneficiary, snapshot_id: x }.into());
	}

	// pass
	shutdown {
		let origin = T::GovernanceOrigin::successful_origin();
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::shutdown {};
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_eq!(<ExchangeState<T>>::get(), false);
		assert_eq!(<IngressMessages<T>>::get().last().unwrap(), &polkadex_primitives::ingress::IngressMessages::Shutdown);
	}

	// pass
	set_exchange_state {
		let x in 0 .. 100_000;
		let state = x % 2 == 0;
		let origin = T::GovernanceOrigin::successful_origin();
		<ExchangeState<T>>::put(state);
		let call = Call::<T>::set_exchange_state { state: !state };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_eq!(<ExchangeState<T>>::get(), !state);
	}

	// PERMABLOCKS
	claim_withdraw {
		let x in 0 .. 100_000;
		let origin = T::EnclaveOrigin::successful_origin();
		let main_origin = T::EnclaveOrigin::successful_origin();
		let main: T::AccountId = match unsafe { main_origin.clone().into().unwrap_unchecked() } {
			RawOrigin::Signed(account) => account.into(),
			_ => panic!("wrong RawOrigin returned")
		};
		let asset = AssetId::asset(x.into());
		let amount = BalanceOf::<T>::decode(&mut &(x as u128).to_le_bytes()[..]).unwrap();
		let mut withdrawals = Vec::with_capacity(1);
		let fees = Decimal::new(100, 1);
		withdrawals.push(Withdrawal {
			amount: Decimal::new(x.into(), 0),
			asset,
			main_account: main.clone(),
			event_id: 1,
			fees,
		});
		let withdrawals: BoundedVec<Withdrawal<T::AccountId>, WithdrawalLimit> = frame_support::BoundedVec::try_from(withdrawals).unwrap();
		let mut wm = BoundedBTreeMap::new();
		wm.try_insert(main.clone(), withdrawals.clone()).unwrap();
		<ExchangeState<T>>::put(true);
		<Withdrawals<T>>::insert(x, wm);
		let call = Call::<T>::claim_withdraw { snapshot_id: x, account: main.clone() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::WithdrawalClaimed {
			main,
			withdrawals,
		}.into());
	}

	// pass
	register_enclave {
		let x in 0 .. 65_000;
		let origin = T::EnclaveOrigin::successful_origin();
		let signer: T::AccountId = T::AccountId::decode(&mut &TEST4_SETUP.signer_pub[..]).unwrap();
		<AllowlistedEnclaves<T>>::insert(&signer, true);
		let call = Call::<T>::register_enclave { ias_report: TEST4_SETUP.cert.to_vec() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::EnclaveRegistered(signer).into());
	}

	// pass
	allowlist_token {
		let x in 0 .. 65_000;
		let origin = T::GovernanceOrigin::successful_origin();
		let asset_id = AssetId::asset(x.into());
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::allowlist_token { token: asset_id };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::TokenAllowlisted(asset_id).into());
	}

	// pass
	remove_allowlisted_token {
		let x in 0 .. 65_000;
		let origin = T::GovernanceOrigin::successful_origin();
		let asset_id = AssetId::asset(x.into());
		let mut at: BoundedBTreeSet<AssetId, AllowlistedTokenLimit> = BoundedBTreeSet::new();
		at.try_insert(asset_id).unwrap();
		<AllowlistedToken<T>>::put(at);
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::remove_allowlisted_token { token: asset_id };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::AllowlistedTokenRemoved(asset_id).into());
	}

	// pass
	allowlist_enclave {
		let x in 0 .. 255; // should not overflow u8
		let origin = T::GovernanceOrigin::successful_origin();
		let account = T::AccountId::decode(&mut &[x as u8; 32].to_vec()[..]).unwrap();
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::allowlist_enclave { enclave_account_id: account.clone() };
	}: { call.dispatch_bypass_filter(origin)? }
	verify {
		assert_last_event::<T>(Event::EnclaveAllowlisted(account).into());
	}
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(Ocex, crate::mock::new_test_ext(), crate::mock::Test);
=======
// GNU General Public License for more details.

//! Benchmarking for pallet-example-basic.

#![cfg(feature = "runtime-benchmarks")]

use crate::*;
use frame_benchmarking::{allowlisted_caller, benchmarks};
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
>>>>>>> .merge_file_p0ZnrF
