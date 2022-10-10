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
// GNU General Public License for more details.unwrap().

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Benchmarking setup for pallet-ocex
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use crate::Pallet as Ocex;
use codec::{Decode, Encode};
use frame_benchmarking::{account, benchmarks};
use frame_support::{
	dispatch::UnfilteredDispatchable, traits::EnsureOrigin, BoundedBTreeMap, BoundedVec,
};
use frame_system::RawOrigin;
use pallet_timestamp::{self as timestamp};
use polkadex_primitives::{
	ocex::{AccountInfo, TradingPairConfig},
	snapshot::{EnclaveSnapshot, Fees},
	withdrawal::Withdrawal,
	ProxyLimit, WithdrawalLimit, UNIT_BALANCE,
};
use rust_decimal::{prelude::*, Decimal};
use sp_core::{crypto::Pair as PairTrait, H256};
use sp_runtime::{traits::CheckedConversion, BoundedBTreeSet};
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
		<ExchangeState<T>>::put(true);
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

	// panci!
	submit_snapshot {
		let x in 0 .. 255; // should not overflow u8
		let pair = sp_core::sr25519::Pair::from_seed(&[x as u8; 32]);
		let public = pair.public();
		let origin = T::AccountId::decode(&mut public.0.as_slice()).unwrap();
		let snapshot = EnclaveSnapshot {
			snapshot_number: x,
			snapshot_hash: H256::from([x as u8; 32]),
			withdrawals: Default::default(),
			fees: Default::default()
		};
		let bytes = snapshot.encode();
		let signature = T::Signature::decode(&mut pair.sign(&bytes).0.as_slice()).unwrap();
		<ExchangeState<T>>::put(true);
		let call = Call::<T>::submit_snapshot { snapshot, signature };
	}: { call.dispatch_bypass_filter(RawOrigin::Signed(origin).into())? }
	verify {
		assert!(<Snapshots<T>>::contains_key(x));
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
				snapshot_hash: H256::from([x as u8; 32]),
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
		let amount = BalanceOf::<T>::decode(&mut &(x as u128).to_be_bytes()[..]).unwrap();
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
		<ExchangeState<T>>::put(true);
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
