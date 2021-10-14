// This file is part of Polkadex.

// Copyright (C) 2020-2021 Polkadex oü.
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

use frame_benchmarking::{account, benchmarks, impl_benchmark_test_suite, whitelist_account};
use frame_support::traits::{EnsureOrigin, Get, UnfilteredDispatchable};
use frame_system::{self, EventRecord, RawOrigin};
use orml_tokens::{AccountData, Accounts};
use sp_runtime::traits::Bounded;
use sp_runtime::traits::One;

use crate::Pallet as PolkadexIdo;

use super::*;

const SEED: u32 = 0;

fn assert_last_event<T: Config>(generic_event: <T as Config>::Event) {
    let events = frame_system::Pallet::<T>::events();
    let system_event: <T as frame_system::Config>::Event = generic_event.into();
    // compare to the last event record
    let EventRecord { event, .. } = &events[events.len() - 1];
    assert_eq!(event, &system_event);
}

fn set_up<T: Config>(caller: T::AccountId) {
    let currency_id: T::CurrencyId = T::NativeCurrencyId::get();
    let account_data: AccountData<T::Balance> = AccountData {
        free: T::Balance::max_value(),
        reserved: T::Balance::zero(),
        frozen: T::Balance::zero(),
    };

    <Accounts<T>>::insert(caller, currency_id, account_data);
}

benchmarks! {
    register_investor {
        let caller: T::AccountId = account("origin", 0, SEED);
        set_up::<T>(caller.clone());
        whitelist_account!(caller);
    }: _(RawOrigin::Signed(caller.clone()))
    verify {
        assert_last_event::<T>(Event::<T>::InvestorRegistered(caller).into());
    }

    attest_investor {
        let caller: T::AccountId = account("origin", 0, SEED);
        set_up::<T>(caller.clone());
        whitelist_account!(caller);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(caller.clone()).into());
        let call = Call::<T>::attest_investor(caller.clone(), KYCStatus::Tier0);
        let origin = T::GovernanceOrigin::successful_origin();
    }: {call.dispatch_bypass_filter(origin)?}
    verify {
        assert_last_event::<T>(Event::<T>::InvestorAttested(caller).into());
    }

    register_round {
        let caller: T::AccountId = account("origin", 0, SEED);
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
    }: _(RawOrigin::Signed(caller.clone()),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                block_num,
                balance,
                balance,
                balance,
                balance,
                block_num)
    verify {
        ensure!(<InfoProjectTeam<T>>::contains_key(caller.clone()), "Register Funding Round didn't work");
        let round_id = <InfoProjectTeam<T>>::get(caller);
        assert_last_event::<T>(Event::<T>::FundingRoundRegistered(round_id).into());
    }

    whitelist_investor {
        let investor_address: T::AccountId = account("origin", 100, SEED);
        set_up::<T>(investor_address.clone());
        whitelist_account!(investor_address);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
        let caller: T::AccountId = account("origin", 101, SEED);
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
        PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
        let round_id = <InfoProjectTeam<T>>::get(caller.clone());
    }: _(RawOrigin::Signed(caller.clone()), round_id.clone(), investor_address.clone(), T::Balance::max_value())
    verify {
        ensure!(<WhiteListInvestors<T>>::contains_key(&round_id.clone(), investor_address.clone()), "WhiteListInvestors didn't work");
        assert_last_event::<T>(Event::<T>::InvestorWhitelisted(round_id, investor_address).into());
    }

    participate_in_round {
        let investor_address: T::AccountId = account("origin", 102, SEED);
        set_up::<T>(investor_address.clone());
        whitelist_account!(investor_address);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
        assert_eq!(<InfoInvestor<T>>::contains_key(investor_address.clone()), true);
        let caller: T::AccountId = account("origin", 103, SEED);
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
        PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
        let round_id = <InfoProjectTeam<T>>::get(caller.clone());
        assert_eq!(<InfoProjectTeam<T>>::contains_key(caller.clone()), true);
        assert_eq!(<InfoFundingRound<T>>::contains_key(round_id.clone()), true);
        PolkadexIdo::<T>::whitelist_investor(
            RawOrigin::Signed(caller.clone()).into(),
            round_id.clone(),
            investor_address.clone(),
            T::Balance::from(100u32)
        );
    }: _(RawOrigin::Signed(investor_address.clone()), round_id.clone(), T::Balance::from(1000u32))
    verify {
        ensure!(<InvestorShareInfo<T>>::contains_key(&round_id.clone(), investor_address.clone()), "ParticipatedInRound didn't work");
        assert_last_event::<T>(Event::<T>::ParticipatedInRound(round_id, investor_address).into());
    }

    claim_tokens {
        let investor_address: T::AccountId = account("origin", 104, SEED);
        set_up::<T>(investor_address.clone());
        whitelist_account!(investor_address);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
        assert_eq!(<InfoInvestor<T>>::contains_key(investor_address.clone()), true);
        let caller: T::AccountId = account("origin", 105, SEED);
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
        PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
        let round_id = <InfoProjectTeam<T>>::get(caller.clone());
        assert_eq!(<InfoProjectTeam<T>>::contains_key(caller.clone()), true);
        assert_eq!(<InfoFundingRound<T>>::contains_key(round_id.clone()), true);
        PolkadexIdo::<T>::whitelist_investor(
            RawOrigin::Signed(caller.clone()).into(),
            round_id.clone(),
            investor_address.clone(),
            T::Balance::from(100u32)
        );
        PolkadexIdo::<T>::claim_tokens(
            RawOrigin::Signed(caller.clone()).into(),
            round_id.clone(),
        );
    }: _(RawOrigin::Signed(investor_address.clone()), round_id.clone())
    verify {
        ensure!(<LastClaimBlockInfo<T>>::contains_key(&round_id.clone(), investor_address.clone()), "Claim Token didn't work");
        assert_last_event::<T>(Event::<T>::TokenClaimed(round_id, investor_address).into());
    }

    show_interest_in_round {
        let investor_address: T::AccountId = account("origin", 105, SEED);
        set_up::<T>(investor_address.clone());
        whitelist_account!(investor_address);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
        assert_eq!(<InfoInvestor<T>>::contains_key(investor_address.clone()), true);
        let caller: T::AccountId = account("origin", 106, SEED);
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
        PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
        let round_id = <InfoProjectTeam<T>>::get(caller.clone());
    }: _(RawOrigin::Signed(investor_address.clone()), round_id.clone())
    verify {
        ensure!(<InterestedParticipants<T>>::contains_key(&round_id.clone()), "ShowedInterest didn't work");
        assert_last_event::<T>(Event::<T>::ShowedInterest(round_id, investor_address).into());
    }

    withdraw_raise {
        let investor_address: T::AccountId = account("origin", 106, SEED);
        set_up::<T>(investor_address.clone());
        whitelist_account!(investor_address);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
        assert_eq!(<InfoInvestor<T>>::contains_key(investor_address.clone()), true);
        let caller: T::AccountId = account("origin", 107, SEED);
        set_up::<T>(caller.clone());
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
        PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
        let round_id = <InfoProjectTeam<T>>::get(caller.clone());
    }: _(RawOrigin::Signed(caller.clone()), round_id.clone(), investor_address)
    verify {
        assert_last_event::<T>(Event::<T>::WithdrawRaised(round_id, caller).into());
    }

    withdraw_token {
        let investor_address: T::AccountId = account("origin", 106, SEED);
        set_up::<T>(investor_address.clone());
        whitelist_account!(investor_address);
        PolkadexIdo::<T>::register_investor(RawOrigin::Signed(investor_address.clone()).into());
        assert_eq!(<InfoInvestor<T>>::contains_key(investor_address.clone()), true);
        let caller: T::AccountId = account("origin", 107, SEED);
        set_up::<T>(caller.clone());
        whitelist_account!(caller);
        let balance = T::Balance::one();
        let block_num = T::BlockNumber::one();
        PolkadexIdo::<T>::register_round(RawOrigin::Signed(caller.clone()).into(),
                AssetId::POLKADEX,
                balance,
                AssetId::POLKADEX,
                balance,
                T::BlockNumber::zero() ,
                balance,
                balance,
                balance,
                balance,
                <frame_system::Pallet<T>>::block_number() + T::BlockNumber::from(32u32));
        let round_id = <InfoProjectTeam<T>>::get(caller.clone());
    }: _(RawOrigin::Signed(caller.clone()), round_id.clone(), investor_address)
    verify {
        assert_last_event::<T>(Event::<T>::WithdrawToken(round_id, caller).into());
    }
}

impl_benchmark_test_suite!(
    PolkadexIdo,
    crate::mock::ExtBuilder::default().build(),
    crate::mock::Test,
);
