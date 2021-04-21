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

use frame_system as system;
use frame_system::{ensure_signed};
use frame_support::StorageMap;
use sp_std::prelude::*;
use sp_runtime::traits::AccountIdConversion;
use frame_support::{
    decl_error, decl_event, decl_module,
    dispatch::DispatchResult,
    ensure,
    traits::Get, PalletId
};
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use polkadex_primitives::assets::{AssetId};
// pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config: system::Config + orml_tokens::Config + pallet_substratee_registry::Config {
    /// Events
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    /// Bonding Account
    type OcexId: Get<PalletId>;
    /// Currency for transfer currencies
    type Currency: MultiCurrencyExtended<Self::AccountId, CurrencyId = AssetId , Balance = Self::Balance>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
        <T as orml_tokens::Config>::Balance
    {
        TokenDeposited(AssetId, AccountId, Balance),
        TokenWithdrawn(AssetId, AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        NotARegisteredEnclave,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Deposit
        #[weight = 10000]
        pub fn deposit(origin, asset_id:  AssetId, amount: T::Balance) -> DispatchResult{
            let from: T::AccountId = ensure_signed(origin)?;
            <T as Config>::Currency::transfer(asset_id, &from, &Self::get_account(), amount)?;
            Self::deposit_event(RawEvent::TokenDeposited(asset_id, from, amount));
            Ok(())
        }

        /// Withdraw
        #[weight = 10000]
        pub fn withdraw(origin, asset_id:  AssetId, to: T::AccountId,amount: T::Balance) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(pallet_substratee_registry::EnclaveIndex::<T>::contains_key(&sender), Error::<T>::NotARegisteredEnclave);
            <T as Config>::Currency::transfer(asset_id, &Self::get_account(), &to, amount)?;
            Self::deposit_event(RawEvent::TokenWithdrawn(asset_id, to, amount));
            Ok(())
        }

    }
}

impl<T: Config> Module<T> {
    pub fn get_account() -> T::AccountId {
        T::OcexId::get().into_account()
    }
}
