// This file is part of Substrate.

// Copyright (C) 2017-2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Default weights of pallet-staking.
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 2.0.0-rc6

#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::weights::{constants::RocksDbWeight as DbWeight, Weight};

pub struct WeightInfo;

impl pallet_staking::WeightInfo for WeightInfo {
    fn bond() -> Weight {
        (144278000_u64)
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(4_u64))
    }
    fn bond_extra() -> Weight {
        (110715000_u64)
            .saturating_add(DbWeight::get().reads(4_u64))
            .saturating_add(DbWeight::get().writes(2_u64))
    }
    fn unbond() -> Weight {
        (99840000_u64)
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn withdraw_unbonded_update(s: u32) -> Weight {
        (100728000_u64)
            .saturating_add((63000_u64).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(5_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn withdraw_unbonded_kill(s: u32) -> Weight {
        (168879000_u64)
            .saturating_add((6666000_u64).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(7_u64))
            .saturating_add(DbWeight::get().writes(8_u64))
            .saturating_add(DbWeight::get().writes((1_u64).saturating_mul(s as Weight)))
    }
    fn validate() -> Weight {
        (35539000_u64)
            .saturating_add(DbWeight::get().reads(2_u64))
            .saturating_add(DbWeight::get().writes(2_u64))
    }

    fn kick(_k: u32) -> u64 {
        todo!()
    }

    fn nominate(n: u32) -> Weight {
        (48596000_u64)
            .saturating_add((308000_u64).saturating_mul(n as Weight))
            .saturating_add(DbWeight::get().reads(3_u64))
            .saturating_add(DbWeight::get().writes(2_u64))
    }
    fn chill() -> Weight {
        (35144000_u64)
            .saturating_add(DbWeight::get().reads(2_u64))
            .saturating_add(DbWeight::get().writes(2_u64))
    }
    fn set_payee() -> Weight {
        (24255000_u64)
            .saturating_add(DbWeight::get().reads(1_u64))
            .saturating_add(DbWeight::get().writes(1_u64))
    }
    fn set_controller() -> Weight {
        (52294000_u64)
            .saturating_add(DbWeight::get().reads(3_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn set_validator_count() -> Weight {
        (5185000_u64).saturating_add(DbWeight::get().writes(1_u64))
    }
    fn force_no_eras() -> Weight {
        (5907000_u64).saturating_add(DbWeight::get().writes(1_u64))
    }
    fn force_new_era() -> Weight {
        (5917000_u64).saturating_add(DbWeight::get().writes(1_u64))
    }
    fn force_new_era_always() -> Weight {
        (5952000_u64).saturating_add(DbWeight::get().writes(1_u64))
    }
    fn set_invulnerables(v: u32) -> Weight {
        (6324000_u64)
            .saturating_add((9000_u64).saturating_mul(v as Weight))
            .saturating_add(DbWeight::get().writes(1_u64))
    }
    fn force_unstake(s: u32) -> Weight {
        (119691000_u64)
            .saturating_add((6681000_u64).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(4_u64))
            .saturating_add(DbWeight::get().writes(8_u64))
            .saturating_add(DbWeight::get().writes((1_u64).saturating_mul(s as Weight)))
    }
    fn cancel_deferred_slash(s: u32) -> Weight {
        (5820201000_u64)
            .saturating_add((34672000_u64).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(1_u64))
            .saturating_add(DbWeight::get().writes(1_u64))
    }
    fn payout_stakers_dead_controller(n: u32) -> Weight {
        (0_u64)
            .saturating_add((92486000_u64).saturating_mul(n as Weight))
            .saturating_add(DbWeight::get().reads(4_u64))
            .saturating_add(DbWeight::get().reads((3_u64).saturating_mul(n as Weight)))
            .saturating_add(DbWeight::get().writes((1_u64).saturating_mul(n as Weight)))
    }
    fn payout_stakers_alive_staked(n: u32) -> Weight {
        (0_u64)
            .saturating_add((117324000_u64).saturating_mul(n as Weight))
            .saturating_add(DbWeight::get().reads((5_u64).saturating_mul(n as Weight)))
            .saturating_add(DbWeight::get().writes((3_u64).saturating_mul(n as Weight)))
    }
    fn rebond(l: u32) -> Weight {
        (71316000_u64)
            .saturating_add((142000_u64).saturating_mul(l as Weight))
            .saturating_add(DbWeight::get().reads(4_u64))
            .saturating_add(DbWeight::get().writes(3_u64))
    }
    fn set_history_depth(e: u32) -> Weight {
        (0_u64)
            .saturating_add((51901000_u64).saturating_mul(e as Weight))
            .saturating_add(DbWeight::get().reads(2_u64))
            .saturating_add(DbWeight::get().writes(4_u64))
            .saturating_add(DbWeight::get().writes((7_u64).saturating_mul(e as Weight)))
    }
    fn reap_stash(s: u32) -> Weight {
        (147166000_u64)
            .saturating_add((6661000_u64).saturating_mul(s as Weight))
            .saturating_add(DbWeight::get().reads(4_u64))
            .saturating_add(DbWeight::get().writes(8_u64))
            .saturating_add(DbWeight::get().writes((1_u64).saturating_mul(s as Weight)))
    }
    fn new_era(v: u32, n: u32) -> Weight {
        (0_u64)
            .saturating_add((1440459000_u64).saturating_mul(v as Weight))
            .saturating_add((182580000_u64).saturating_mul(n as Weight))
            .saturating_add(DbWeight::get().reads(10_u64))
            .saturating_add(DbWeight::get().reads((4_u64).saturating_mul(v as Weight)))
            .saturating_add(DbWeight::get().reads((3_u64).saturating_mul(n as Weight)))
            .saturating_add(DbWeight::get().writes(8_u64))
            .saturating_add(DbWeight::get().writes((3_u64).saturating_mul(v as Weight)))
    }

    fn get_npos_voters(_v: u32, _n: u32, _s: u32) -> u64 {
        todo!()
    }

    fn get_npos_targets(_v: u32) -> u64 {
        todo!()
    }
}
