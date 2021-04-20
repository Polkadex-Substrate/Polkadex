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
use frame_system as system;
use frame_system::{ensure_signed};
use sp_runtime::SaturatedConversion;
use sp_runtime::traits::Hash;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;
use sp_runtime::traits::AccountIdConversion;
use orml_traits::arithmetic::{CheckedAdd, CheckedSub};
use orml_tokens::Call::transfer;
use orml_traits::{BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency, MultiCurrency};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{EnsureOrigin, Get, Randomness}, PalletId
};
pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

pub trait Config: system::Config + orml_tokens::Config + pallet_substratee_registry::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type OcexId: Get<PalletId>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
        <T as orml_tokens::Config>::CurrencyId,
        <T as orml_tokens::Config>::Balance
    {
        TokenDeposited(CurrencyId, AccountId, Balance),
        TokenWithdrawn(CurrencyId, AccountId, Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        NotPresentInEnclave,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Deposit
        #[weight = 10000]
        pub fn deposit(origin, asset_id:  T::CurrencyId, amount: T::Balance) -> DispatchResult{
            let from: T::AccountId = ensure_signed(origin)?;
            //orml_tokens::Pallet::<T>::MultiCurrency::transfer(asset_id, &from, &Self::get_account(), amount);
            Self::deposit_event(RawEvent::TokenDeposited(asset_id, from, amount));
            Ok(())
        }

        /// Withdraw
        #[weight = 10000]
        pub fn deposit(origin, asset_id:  T::CurrencyId, to: T::AccountId,amount: T::Balance) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(pallet_substratee_registry::EnclaveIndex::<T>::contains_key(&sender), Error::<T>::NotPresentInEnclave);
            //orml_tokens::Pallet::<T>::MultiCurrency::transfer(asset_id, &Self::get_account(), &to, amount);
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
