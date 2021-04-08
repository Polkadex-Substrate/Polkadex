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
use sp_std::prelude::*;

use orml_traits::{
    BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    type TreasuryAccountId: Get<Self::AccountId>;

    type NativeCurrency: BasicCurrencyExtended<Self::AccountId, Balance = BalanceOf<Self>>
    + BasicLockableCurrency<Self::AccountId, Balance = BalanceOf<Self>>
    + BasicReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>;

    type NativeCurrencyId: Get<Self::CurrencyId>;
    type IDOPDXAmount: Get<Self::Balance>;
    type MaxSupply: Get<Self::Balance>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum KYCStatus {
    Tier0,
    Tier1,
    Tier2
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct InvestorInfo {
    pub kyc_status: KYCStatus,
}

impl Default for InvestorInfo {
    fn default() -> Self {
        InvestorInfo {
            kyc_status: KYCStatus::Tier0,
        }
    }
}

impl InvestorInfo {
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
        InfoInvestor get(fn get_investorinfo): map hasher(identity) T::AccountId => InvestorInfo;
    }
    add_extra_genesis {
		config(endowed_accounts): Vec<(T::AccountId, T::CurrencyId, T::Balance)>;

		build(|config: &GenesisConfig<T>| {
			config.endowed_accounts.iter().for_each(|(account_id, currency_id, initial_balance)| {
				 orml_tokens::Accounts::<T>::mutate(account_id, currency_id, |account_data| account_data.free = *initial_balance)
			})
		})
	}
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
    {
        InvestorRegistered(AccountId),
        InvestorAttested(AccountId),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        InvestorAlreadyRegistered,
        InvestorDoesNotExist,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        #[weight = 10000]
        pub fn register_investor(origin) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            let amount: T::Balance = T::IDOPDXAmount::get();
            ensure!(!<InfoInvestor<T>>::contains_key(&who.clone()), Error::<T>::InvestorAlreadyRegistered);
            if T::NativeCurrency::total_issuance() > T::MaxSupply::get()
            {
                 orml_tokens::Accounts::<T>::mutate(who.clone(), &T::NativeCurrencyId::get(), |account_data| {
                    account_data.free = account_data.free - amount;
                });
            }
            else {
                T::NativeCurrency::transfer(&who, &T::TreasuryAccountId::get(), amount)?;
            }
            let investor_info = InvestorInfo::default();
            <InfoInvestor<T>>::insert(who.clone(), investor_info);
            Self::deposit_event(RawEvent::InvestorRegistered(who));

            Ok(())
        }

        #[weight = 10000]
        pub fn attest_investor(origin, investor: T::AccountId, kyc_status: KYCStatus) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor), <Error<T>>::InvestorDoesNotExist);
            InfoInvestor::<T>::try_mutate(&investor, |ref mut investor_info| {
                investor_info.kyc_status = kyc_status;
                Self::deposit_event(RawEvent::InvestorAttested(investor.clone()));
                Ok(())
            })
        }

    }
}

