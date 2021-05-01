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
use frame_support::StorageMap;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::Get, PalletId,
};
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;

use polkadex_primitives::assets::AssetId;

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct LinkedAccount<AccountID> {
    prev: AccountID,
    next: Option<AccountID>,
    proxies: Vec<AccountID>,
}

impl<AccountId: Default> Default for LinkedAccount<AccountId> {
    fn default() -> Self {
        LinkedAccount {
            prev: AccountId::default(),
            next: None,
            proxies: vec![],
        }
    }
}

pub trait Config:
    system::Config + orml_tokens::Config + pallet_substratee_registry::Config
{
    /// Events
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    /// Bonding Account
    type OcexId: Get<PalletId>;
    /// Currency for transfer currencies
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId = AssetId,
        Balance = Self::Balance,
    >;
    type ProxyLimit: Get<usize>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
        <T as orml_tokens::Config>::Balance
    {
        TokenDeposited(AssetId, AccountId, Balance),
        TokenWithdrawn(AssetId, AccountId, Balance),
        MainAccountRegistered(AccountId),
        ProxyAdded(AccountId,AccountId),
        ProxyRemoved(AccountId,AccountId),
    }
);

// TODO: Implement a vec of MRENCLAVES set by governance

decl_error! {
    pub enum Error for Module<T: Config> {
        NotARegisteredEnclave,
        AlreadyRegistered,
        NotARegisteredMainAccount,
        ProxyLimitReached
    }
}

decl_storage! {
    trait Store for Module<T: Config> as OCEX {
        pub FirstAccount: Option<T::AccountId> = None;
        pub LastAccount: Option<T::AccountId> = None;
        pub MainAccounts get(fn get_main_accounts): map hasher(blake2_128_concat) T::AccountId => LinkedAccount<T::AccountId>;
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

        /// Release
        #[weight = 10000]
        pub fn release(origin, asset_id:  AssetId, amount: T::Balance, to: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(pallet_substratee_registry::EnclaveIndex::<T>::contains_key(&sender), Error::<T>::NotARegisteredEnclave);
            // TODO: Check if the latest MRENCLAVE is registered by this sender
            // TODO: Handle software updated to enclave
            <T as Config>::Currency::transfer(asset_id, &Self::get_account(), &to, amount)?;
            Ok(())
        }

        /// Withdraw
        /// It helps to notify enclave about sender's intend to withdraw via on-chain
        #[weight = 10000]
        pub fn withdraw(origin, asset_id:  AssetId, to: T::AccountId,amount: T::Balance) -> DispatchResult{
            let _: T::AccountId = ensure_signed(origin)?;
            Self::deposit_event(RawEvent::TokenWithdrawn(asset_id, to, amount));
            Ok(())
        }

        /// Register MainAccount
        #[weight = 10000]
        pub fn register(origin) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(!<MainAccounts<T>>::contains_key(&sender), Error::<T>::AlreadyRegistered);
            Self::register_acc(sender.clone())?;
            Self::deposit_event(RawEvent::MainAccountRegistered(sender));
            Ok(())
        }

        /// Add Proxy
        #[weight = 10000]
        pub fn add_proxy(origin, proxy: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(<MainAccounts<T>>::contains_key(&sender), Error::<T>::NotARegisteredMainAccount);
            Self::add_proxy_(sender.clone(),proxy.clone())?;
            Self::deposit_event(RawEvent::ProxyAdded(sender,proxy));
            Ok(())
        }

        /// Remove Proxy
        #[weight = 10000]
        pub fn remove_proxy(origin, proxy: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(<MainAccounts<T>>::contains_key(&sender), Error::<T>::NotARegisteredMainAccount);
            Self::remove_proxy_(sender.clone(),proxy.clone())?;
            Self::deposit_event(RawEvent::ProxyRemoved(sender,proxy));
            Ok(())
        }

    }
}

impl<T: Config> Module<T> {
    // Note add_proxy doesn't check if given proxy is already registered
    pub fn add_proxy_(main: T::AccountId, proxy: T::AccountId) -> Result<(), Error<T>> {
        let mut acc: LinkedAccount<T::AccountId> = <MainAccounts<T>>::get(&main);
        if acc.proxies.len() < T::ProxyLimit::get() {
            acc.proxies.push(proxy);
            <MainAccounts<T>>::insert(main, acc);
        } else {
            return Err(Error::<T>::ProxyLimitReached);
        }
        Ok(())
    }

    pub fn remove_proxy_(main: T::AccountId, proxy: T::AccountId) -> Result<(), Error<T>> {
        let mut acc: LinkedAccount<T::AccountId> = <MainAccounts<T>>::get(&main);
        for i in 0..T::ProxyLimit::get() {
            match acc.proxies.get(i) {
                None => {}
                Some(registered_proxy) => {
                    if registered_proxy.eq(&proxy) {
                        acc.proxies.remove(i);
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn get_account() -> T::AccountId {
        T::OcexId::get().into_account()
    }

    pub fn register_acc(sender: T::AccountId) -> Result<(), Error<T>> {
        match <FirstAccount<T>>::get() {
            Some(_) => {
                // Get current last account_id
                let last_acc_option: Option<T::AccountId> = <LastAccount<T>>::get();
                let last_acc: T::AccountId = last_acc_option.unwrap();
                // Get current last account
                // If first acc is defined then last acc must be there
                let mut last: LinkedAccount<T::AccountId> = <MainAccounts<T>>::get(&last_acc);
                // modify next of current last account to sender
                last.next = Some(sender.clone());
                // write back modified previous last account
                <MainAccounts<T>>::insert(&last_acc, last);
                // set sender to last account
                <LastAccount<T>>::put(sender.clone());
                // write sender's struct to LinkedAccount
                <MainAccounts<T>>::insert(
                    sender,
                    LinkedAccount {
                        prev: last_acc,
                        next: None,
                        proxies: vec![],
                    },
                );
            }
            None => {
                // Set sender as first acc
                <FirstAccount<T>>::put(sender.clone());
                // Set sender as last acc as  there is only one acc
                <LastAccount<T>>::put(sender.clone());
                // Set the sender's acc details
                <MainAccounts<T>>::insert(
                    sender,
                    LinkedAccount {
                        prev: T::AccountId::default(),
                        next: None,
                        proxies: vec![],
                    },
                );
            }
        }
        Ok(())
    }
}
