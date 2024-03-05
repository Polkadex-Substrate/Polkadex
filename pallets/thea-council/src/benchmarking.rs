// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

use super::*;

#[allow(unused)]
use crate::Pallet as TheaCouncil;
use frame_benchmarking::v1::{account, benchmarks};
use frame_support::{sp_runtime::SaturatedConversion, BoundedVec};
use frame_system::{pallet_prelude::BlockNumberFor, RawOrigin};
use sp_std::{vec, vec::Vec};
use thea_primitives::types::Withdraw;
const SEED: u32 = 0;

benchmarks! {
    add_member {
        // Add sender to council member
        let b in 1 .. 1000;
        let council_member: T::AccountId = account("mem1", b, SEED);
        let mut active_council_member = <ActiveCouncilMembers<T>>::get();
        active_council_member.try_push(council_member.clone()).unwrap();
        <ActiveCouncilMembers<T>>::put(active_council_member);
        let new_member: T::AccountId = account("mem2", b, SEED);
    }: _(RawOrigin::Signed(council_member.clone()), new_member)
    verify {
        let active_members = <ActiveCouncilMembers<T>>::get();
        assert!(active_members.contains(&council_member));
    }

    remove_member {
        let b in 1 .. 1000;
        let first_council_member: T::AccountId = account("mem1", b, SEED);
        let sec_council_member: T::AccountId = account("mem2", b, SEED);
        let third_council_member: T::AccountId = account("mem3", b, SEED);
        let mut active_council_member = <ActiveCouncilMembers<T>>::get();
        active_council_member.try_push(first_council_member.clone()).unwrap();
        active_council_member.try_push(sec_council_member.clone()).unwrap();
        active_council_member.try_push(third_council_member.clone()).unwrap();
        <ActiveCouncilMembers<T>>::put(active_council_member);
        let proposal = Proposal::RemoveExistingMember(third_council_member.clone());
        let votes = BoundedVec::try_from(vec![Voted(first_council_member)]).unwrap();
        <Proposals<T>>::insert(proposal, votes);
    }: _(RawOrigin::Signed(sec_council_member), third_council_member.clone())
    verify {
        let active_members = <ActiveCouncilMembers<T>>::get();
        assert!(!active_members.contains(&third_council_member));
    }

    claim_membership {
        let b in 1 .. 1000;
        let pending_council_member: T::AccountId = account("mem1", b, SEED);
        let mut pending_council_members = <PendingCouncilMembers<T>>::get();
        pending_council_members.try_push((b.into(),pending_council_member.clone())).unwrap();
        <PendingCouncilMembers<T>>::put(pending_council_members);
    }: _(RawOrigin::Signed(pending_council_member.clone()))
    verify {
        let active_members = <ActiveCouncilMembers<T>>::get();
        assert!(active_members.contains(&pending_council_member));
    }

    delete_transaction {
        let b in 1 .. 1000;
        let council_member: T::AccountId = account("mem1", b, SEED);
        let mut active_council_member = <ActiveCouncilMembers<T>>::get();
        active_council_member.try_push(council_member.clone()).unwrap();
        <ActiveCouncilMembers<T>>::put(active_council_member);
        // Add Pending Withdrawal
        let block_no: BlockNumberFor<T> = 100u64.saturated_into();
        let pending_withdrawal = Withdraw {
            id: Vec::new(),
            asset_id: 0,
            amount: 0,
            destination: vec![],
            is_blocked: false,
            extra: vec![]
        };
        xcm_helper::Pallet::<T>::insert_pending_withdrawal(block_no, pending_withdrawal);
    }: _(RawOrigin::Signed(council_member), block_no, 0u32)
    verify {
        let pending_withdrawal = xcm_helper::Pallet::<T>::get_pending_withdrawals(block_no).pop().unwrap();
        assert!(pending_withdrawal.is_blocked);
    }
}

#[cfg(test)]
use frame_benchmarking::v1::impl_benchmark_test_suite;

#[cfg(test)]
impl_benchmark_test_suite!(TheaCouncil, crate::mock::new_test_ext(), crate::mock::Test);
