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

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{EnsureOrigin, Get},
};
use frame_system as system;
use frame_system::{ensure_signed};
use sp_runtime::SaturatedConversion;
use sp_runtime::traits::Hash;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

use orml_traits::arithmetic::{CheckedAdd, CheckedSub};
use orml_traits::{
    BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
};

pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;

    type NativeCurrency: BasicCurrencyExtended<Self::AccountId, Balance = BalanceOf<Self>>
    + BasicLockableCurrency<Self::AccountId, Balance = BalanceOf<Self>>
    + BasicReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>;
}

pub enum KYCStatus {
    Tier0,
    Tier1,
    Tier2
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct InvestorInfo<T: Config> {
    pub kyc_status: KYCStatus,
}

impl<T: Config> Default for InvestorInfo<T> {
    fn default() -> Self {
        InvestorInfo {
            kyc_status: KYCStatus::Tier0,
        }
    }
}

impl<T: Config> InvestorInfo<T> {
    fn from(
        kyc_status: KYCStatus,
    ) -> Self {
        InvestorInfo {
            kyc_status,
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as PolkadexIdo {
        InfoInvestor get(fn get_investorinfo): map hasher(identity) T::AccountId => InvestorInfo<T>;
        IDOPDXAmount get(fn get_amount): T::Balance;
    }
}

decl_event!(
    pub enum Event<T>
    {

    }
);

decl_error! {
    pub enum Error for Module<T: Config> {

    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        ///register_investor(origin,): The investor needs to  burn 100 PDEX to participate in the events of Polkadex IDO platform. 100 PDEX will be burned if total supply is greater than 20 million else transferred to treasury.
        #[weight = 10000]
        pub fn register_investor(origin) -> DispatchResult {
            orml_tokens::total_issuance();

            Ok(())
        }

    }
}

