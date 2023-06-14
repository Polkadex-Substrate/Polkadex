// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg(feature = "runtime-benchmarks")]
use super::*;
use crate::Pallet as TheaExecutor;
use parity_scale_codec::Decode;

use frame_benchmarking::v1::{account, benchmarks, whitelisted_caller, BenchmarkError};
use frame_support::{
    ensure,
    traits::{EnsureOrigin, Get},
};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;

benchmarks! {
    set_withdrawal_fee {
        let r in 1 .. 1000;
        let network_id = r as u8;
        let fee = 1_000_000_000_000;
    }: _(RawOrigin::Root, network_id, fee)
}