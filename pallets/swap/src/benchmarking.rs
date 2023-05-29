// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! AMM pallet benchmarking.

#![cfg(feature = "runtime-benchmarks")]

use frame_benchmarking::{
	benchmarks_instance_pallet, impl_benchmark_test_suite, whitelisted_caller,
};
use frame_support::{assert_ok, dispatch::UnfilteredDispatchable, traits::EnsureOrigin};
use frame_system::{self, RawOrigin as SystemOrigin};
use sp_runtime::traits::StaticLookup;
use sp_std::prelude::*;

use crate::{CurrencyId, Pallet as AMM};

use super::*;

pub const DOT: CurrencyId = 11;
pub const SDOT: CurrencyId = 12;
const BASE_ASSET: CurrencyId = SDOT;
const QUOTE_ASSET: CurrencyId = DOT;
const INITIAL_AMOUNT: u128 = 1_000_000_000_000_000;
const ASSET_ID: CurrencyId = 10;
const MINIMUM_LIQUIDITY: u128 = 1_000u128;

fn assert_last_event<T: Config<I>, I: 'static>(generic_event: <T as Config<I>>::RuntimeEvent) {
	frame_system::Pallet::<T>::assert_last_event(generic_event.into());
}

fn initial_set_up<
	T: Config<I> + pallet_assets::Config<AssetId = CurrencyId, Balance = Balance>,
	I: 'static,
>(
	caller: T::AccountId,
) {
	let account_id = T::Lookup::unlookup(caller.clone());

	pallet_assets::Pallet::<T>::force_create(
		SystemOrigin::Root.into(),
		SDOT.into(),
		account_id.clone(),
		true,
		One::one(),
	)
	.ok();

	pallet_assets::Pallet::<T>::force_create(
		SystemOrigin::Root.into(),
		DOT.into(),
		account_id.clone(),
		true,
		One::one(),
	)
	.ok();

	pallet_assets::Pallet::<T>::force_create(
		SystemOrigin::Root.into(),
		ASSET_ID.into(),
		account_id.clone(),
		true,
		One::one(),
	)
	.ok();

	T::Assets::mint_into(BASE_ASSET, &caller, INITIAL_AMOUNT).ok();
	T::Assets::mint_into(QUOTE_ASSET, &caller, INITIAL_AMOUNT).ok();
}

benchmarks_instance_pallet! {
	where_clause {
		where T: pallet_assets::Config<AssetId = CurrencyId, Balance = Balance>
	}

	add_liquidity {
		let caller: T::AccountId = whitelisted_caller();
		initial_set_up::<T, I>(caller.clone());
		let base_amount = 100_000u128;
		let quote_amount = 200_000u128;
		assert_ok!(AMM::<T, I>::create_pool(T::CreatePoolOrigin::successful_origin(),
			(BASE_ASSET, QUOTE_ASSET), (base_amount, quote_amount),
			caller.clone(), ASSET_ID.into()));
	}: _(
		SystemOrigin::Signed(caller.clone()),
		(BASE_ASSET, QUOTE_ASSET),
		(base_amount, quote_amount),
		(5u128, 5u128)
	)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::LiquidityAdded(
			caller,
			BASE_ASSET,
			QUOTE_ASSET,
			base_amount,
			quote_amount,
			ASSET_ID.into(),
			base_amount * 2,
			quote_amount * 2,
		).into());
	}

	remove_liquidity {
		let caller: T::AccountId = whitelisted_caller();
		initial_set_up::<T, I>(caller.clone());
		let base_amount = 100_000u128;
		let quote_amount = 900_000u128;
		assert_ok!(AMM::<T, I>::create_pool(T::CreatePoolOrigin::successful_origin(),
			(BASE_ASSET, QUOTE_ASSET), (base_amount, quote_amount),
			caller.clone(), ASSET_ID.into()));
	}: _(
		SystemOrigin::Signed(caller.clone()),
		(BASE_ASSET, QUOTE_ASSET),
		300_000u128 - MINIMUM_LIQUIDITY
	)
	verify {
		assert_last_event::<T, I>(Event::<T, I>::LiquidityRemoved(
			caller,
			BASE_ASSET,
			QUOTE_ASSET,
			300_000u128 - MINIMUM_LIQUIDITY,
			99666,
			897000,
			ASSET_ID.into(),
			334,
			3000,
		).into());
	}

	create_pool {
		let caller: T::AccountId = whitelisted_caller();
		initial_set_up::<T, I>(caller.clone());
		let base_amount = 100_000u128;
		let quote_amount = 200_000u128;
		let origin = T::CreatePoolOrigin::successful_origin();
		let call = Call::<T, I>::create_pool {
			pair: (BASE_ASSET, QUOTE_ASSET),
			liquidity_amounts: (base_amount, quote_amount),
			lptoken_receiver: caller.clone(),
			lp_token_id: ASSET_ID.into()
		};
	}: {
		call.dispatch_bypass_filter(origin)?
	}
	verify {
		assert_last_event::<T, I>(Event::<T, I>::LiquidityAdded(
			caller,
			BASE_ASSET,
			QUOTE_ASSET,
			base_amount,
			quote_amount,
			ASSET_ID.into(),
			base_amount,
			quote_amount,
		).into());
	}

	update_protocol_fee {
		let origin = T::ProtocolFeeUpdateOrigin::successful_origin();
		let call = Call::<T, I>::update_protocol_fee {
			protocol_fee: Ratio::from_percent(20)
		};
	}: {
		call.dispatch_bypass_filter(origin)?
	}
	verify {
	}

	update_protocol_fee_receiver {
		let caller: T::AccountId = whitelisted_caller();
		let origin = T::ProtocolFeeUpdateOrigin::successful_origin();
		let call = Call::<T, I>::update_protocol_fee_receiver {
			protocol_fee_receiver: caller
		};
	}: {
		call.dispatch_bypass_filter(origin)?
	}
	verify {
	}
}

impl_benchmark_test_suite!(AMM, crate::mock::new_test_ext(), crate::mock::Test,);
