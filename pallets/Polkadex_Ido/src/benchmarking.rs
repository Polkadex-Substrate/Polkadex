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

//! Benchmarking setup for pallet-template

// use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account};
// use frame_support::traits::{EnsureOrigin, Get, UnfilteredDispatchable};
// use frame_system::{self, EventRecord, RawOrigin};
// use orml_tokens::{AccountData, Accounts};
// use sp_runtime::traits::Bounded;
// use sp_runtime::traits::One;
//
// use crate::Pallet as PolkadexIdo;
//
// use super::*;
//
// const SEED: u32 = 0;
//
// fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
//     let events = frame_system::Pallet::<T>::events();
//     let system_event: <T as frame_system::Config>::Event = generic_event.into();
//     // compare to the last event record
//     let EventRecord { event, .. } = &events[events.len() - 1];
//     assert_eq!(event, &system_event);
// }
//
// fn set_up<T: Config>(caller: T::AccountId) {
//     let currency_id: T::CurrencyId = T::NativeCurrencyId::get();
//     let account_data: AccountData<T::Balance> = AccountData {
//         free: T::Balance::max_value(),
//         reserved: T::Balance::zero(),
//         frozen: T::Balance::zero(),
//     };
//
//     <Accounts<T>>::insert(caller, currency_id, account_data);
// }
//
//
//
// impl_benchmark_test_suite!(
//     PolkadexIdo,
//     crate::mock::ExtBuilder::default().build(),
//     crate::mock::Test,
// );
