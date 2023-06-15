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
use frame_support::traits::fungible::Mutate;
use frame_support::traits::fungibles::{Inspect, Mutate};
use frame_system::RawOrigin;
use sp_runtime::traits::Bounded;
use thea_primitives::types::{AssetMetadata, Deposit};
use sp_runtime::SaturatedConversion;


benchmarks! {
    set_withdrawal_fee {
        let r in 1 .. 1000;
        let network_id = r as u8;
        let fee = 1_000_000_000_000;
    }: _(RawOrigin::Root, network_id, fee)
    verify {
        assert_eq!(<WithdrawalFees<T>>::get(network_id), Some(fee));
    }

    update_asset_metadata {
        let r in 1 .. 1000;
        let asset_id = r as u128;
        let decimal: u8 = 8;
    }: _(RawOrigin::Root, asset_id, decimal)
    verify {
        let metadata = AssetMetadata::new(decimal).unwrap();
        assert_eq!(<Metadata<T>>::get(asset_id), Some(metadata));
    }

    claim_deposit {
        let r in 1 .. 1000;
        let account = account::<T::AccountId>("alice", 1, r);
        let deposits = create_deposit::<T>(account.clone());
        <ApprovedDeposits<T>>::insert(account.clone(), deposits);
    }: _(RawOrigin::Signed(account.clone()), 10)
    verify {
        //let current_balance = T::Assets::balance(100, &account);
        //assert_eq!(current_balance.into(), 10_000_000_000_000);
    }

    withdraw {
        let r in 1 .. 1000;
        //Create Asset
        //Mint Tokens
        let account = account::<T::AccountId>("alice", 1, r);
        T::Assets::mint_into(100, account.clone(), 1_000_000_000_000);
        //Mint Native Asset
        T::Currency::mint_into(account_clone(), 1_000_000_000_000);
        //Set Metadata
        let metadata = AssetMetadata::new(decimal).unwrap();
        <Metadata<T>>::insert(100, metadata);
        //Set Withdrawal Fee
        <WithdrawalFees<T>>::insert(1, 1_000);
        let benificary = vec![1;32];
    }: _(RawOrigin::Signed(account.clone()), 100, 1_000_000_000_000, benificary, true)

    parachain_withdraw {
        let r in 1 .. 1000;
        //Create Asset
        //Mint Tokens
        let account = account::<T::AccountId>("alice", 1, r);
        T::Assets::mint_into(100, account.clone(), 1_000_000_000_000);
        //Mint Native Asset
        T::Currency::mint_into(account_clone(), 1_000_000_000_000);
        //Set Metadata
        let metadata = AssetMetadata::new(decimal).unwrap();
        <Metadata<T>>::insert(100, metadata);
        //Set Withdrawal Fee
        <WithdrawalFees<T>>::insert(1, 1_000);
        let benificary = vec![1;32];
    }: _(RawOrigin::Signed(account.clone()), 100, 1_000_000_000_000, benificary, true)
}

fn create_deposit<T:Config>(recipient: T::AccountId) -> Vec<Deposit<T::AccountId>> {
    T::Currency::mint_into()
    let mut pending_deposits = vec![];
    let asset_id = 100;
    for i in 1 .. 10 {
        let deposit: Deposit<T::AccountId> = Deposit{
            id: vec![],
            recipient: recipient.clone(),
            asset_id: asset_id,
            amount: 1_000_000_000_000,
            extra: vec![]
        };
        pending_deposits.push(deposit);
    }
    pending_deposits
}