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
#![allow(clippy::unused_unit)]

use codec::Encode;
use core::convert::TryInto;
use frame_support::StorageMap;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult, ensure,
    traits::Get, PalletId,
};
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use polkadex_primitives::assets::AssetId;
use polkadex_primitives::AccountId;
use polkadex_sgx_primitives::LinkedAccount;
//use sp_runtime::traits::AccountIdConversion;
use sp_std::prelude::*;
use crate::sp_api_hidden_includes_decl_storage::hidden_include::sp_runtime::traits::AccountIdConversion;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config:
    system::Config + orml_tokens::Config + pallet_substratee_registry::Config
{
    /// Events
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    /// Bonding Account
    type OcexId: Get<PalletId>;
    /// LinkedList Genesis Account
    type GenesisAccount: Get<PalletId>;
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
        TokenRelease(AssetId, AccountId, Balance),
        MainAccountRegistered(AccountId),
        ProxyAdded(AccountId,AccountId),
        ProxyRemoved(AccountId,AccountId),
        CIDUploaded(AccountId, Vec<u8>),
    }
);

// TODO: Implement a vec of MRENCLAVES set by governance

decl_error! {
    pub enum Error for Module<T: Config> {
        NotARegisteredEnclave,
        AlreadyRegistered,
        NotARegisteredMainAccount,
        ProxyLimitReached,
        MainAccountSignatureNotFound,
        AccountIdConversionFailed
    }
}

decl_storage! {
    trait Store for Module<T: Config> as OCEX {
        LastAccount get(fn key) config(): T::AccountId;
        pub MainAccounts get(fn get_main_accounts): map hasher(blake2_128_concat) T::AccountId => LinkedAccount;
        pub Snapshot get(fn get_snapshot): map hasher(blake2_128_concat) T::AccountId => Vec<u8>;
    }
    add_extra_genesis {
        config(genesis_account): T::AccountId;
        build( |config: &GenesisConfig<T>| {
           match (config.genesis_account.clone().encode().as_slice().try_into(), config.genesis_account.clone().encode().as_slice().try_into()) {
                (Ok(x),Ok(y)) => {
                    let linked_account_object = LinkedAccount::from(AccountId::new(x), AccountId::new(y) );
                     <MainAccounts<T>>::insert(&config.genesis_account, linked_account_object);
                }
                (_,_) => {}
            }
        });
    }
}
decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;


    /// Transfers given amount to Enclave.
        ///
        /// # Parameters
        ///
        /// * `main`: Account from which amount is to be transferred
        /// * `asset_id`: Asset Id
        /// * `amount`: Amount to be transferred to Enclave
        #[weight = 10000]
        pub fn deposit(origin, main: T::AccountId, asset_id:  AssetId, amount: T::Balance) -> DispatchResult{
            let from: T::AccountId = ensure_signed(origin)?;
            ensure!(main==from, Error::<T>::MainAccountSignatureNotFound);
            <T as Config>::Currency::transfer(asset_id, &from, &Self::get_account(), amount)?;
            Self::deposit_event(RawEvent::TokenDeposited(asset_id, from, amount));
            Ok(())
        }

    /// Releases/Transfers given amount to Destination Account,
        /// Only Enclave can call this Dispatchable function.
        ///
        /// # Parameters
        ///
        /// * `asset_id`: Asset Id
        /// * `amount`: Amount to be released
        /// * `to`: Destination Account
        #[weight = 10000]
        pub fn release(origin, asset_id:  AssetId, amount: T::Balance, to: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(pallet_substratee_registry::EnclaveIndex::<T>::contains_key(&sender), Error::<T>::NotARegisteredEnclave);
            // TODO: Check if the latest MRENCLAVE is registered by this sender
            // TODO: Handle software updated to enclave
            <T as Config>::Currency::transfer(asset_id, &Self::get_account(), &to, amount)?;
            Self::deposit_event(RawEvent::TokenRelease(asset_id, to, amount));
            Ok(())
        }

        /// Notifies enclave about sender's intend to withdraw via on-chain.
        ///
        /// # Parameters
        ///
        /// * `main`: Account which wants to Notify Enclave
        /// * `asset_id`: Asset Id
        /// * `amount`: Amount to be notified to Enclave
        #[weight = 10000]
        pub fn withdraw(origin,  main: T::AccountId, asset_id:  AssetId, amount: T::Balance) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(main==sender, Error::<T>::MainAccountSignatureNotFound);
            Self::deposit_event(RawEvent::TokenWithdrawn(asset_id, sender, amount));
            Ok(())
        }

    /// Registers main Account.
        ///
        /// # Parameter
        ///
        /// * `main`: Main Account to be registered
        #[weight = 10000]
        pub fn register(origin, main: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(main==sender, Error::<T>::MainAccountSignatureNotFound);
            ensure!(!<MainAccounts<T>>::contains_key(&sender), Error::<T>::AlreadyRegistered);
            Self::register_acc(sender.clone())?;
            Self::deposit_event(RawEvent::MainAccountRegistered(sender));
            Ok(())
        }

    /// Adds Proxy Account for given Main Account.
        ///
        /// # Parameter
        ///
        /// * `main`: Main Account for which Proxy Account is to be added
        /// * `proxy`: Proxy Account to be added
        #[weight = 10000]
        pub fn add_proxy(origin, main: T::AccountId, proxy: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(main==sender, Error::<T>::MainAccountSignatureNotFound);
            ensure!(<MainAccounts<T>>::contains_key(&sender), Error::<T>::NotARegisteredMainAccount);
            Self::add_proxy_(sender.clone(),proxy.clone())?;
            Self::deposit_event(RawEvent::ProxyAdded(sender,proxy));
            Ok(())
        }

    /// Removes Proxy Account for given Main Account.
        ///
        /// # Parameter
        ///
        /// * `main`: Main Account for which Proxy Account is to be removed
        /// * `proxy`: Proxy Account to be removed
        #[weight = 10000]
        pub fn remove_proxy(origin, main: T::AccountId, proxy: T::AccountId) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(main==sender, Error::<T>::MainAccountSignatureNotFound);
            ensure!(<MainAccounts<T>>::contains_key(&sender), Error::<T>::NotARegisteredMainAccount);
            Self::remove_proxy_(sender.clone(),proxy.clone())?;
            Self::deposit_event(RawEvent::ProxyRemoved(sender,proxy));
            Ok(())
        }

        /// Uploads CID for given Enclave id
        ///
        /// # Parameter
        ///
        /// * `new_cid`: CID to be uploaded for given Enclave Id
        #[weight = 10000]
        pub fn upload_cid(origin, new_cid: Vec<u8>) -> DispatchResult{
            let sender: T::AccountId = ensure_signed(origin)?;
            ensure!(pallet_substratee_registry::EnclaveIndex::<T>::contains_key(&sender), Error::<T>::NotARegisteredEnclave);
            <Snapshot<T>>::try_mutate(sender.clone(), |ref mut old_cid| {
                **old_cid = new_cid.clone();
                Self::deposit_event(RawEvent::CIDUploaded(sender,new_cid));
                Ok(())
            })
        }
    }
}

impl<T: Config> Module<T> {
    // Note add_proxy doesn't check if given main or proxy is already registered
    pub fn add_proxy_(main: T::AccountId, proxy: T::AccountId) -> Result<(), Error<T>> {
        let mut acc: LinkedAccount = <MainAccounts<T>>::get(&main);
        if acc.proxies.len() < T::ProxyLimit::get() {
            acc.proxies.push(AccountId::new(
                proxy
                    .encode()
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::AccountIdConversionFailed)?,
            ));
            <MainAccounts<T>>::insert(main, acc);
        } else {
            return Err(Error::<T>::ProxyLimitReached);
        }
        Ok(())
    }

    // Note remove_proxy doesn't check if given main or proxy is already registered
    pub fn remove_proxy_(main: T::AccountId, proxy: T::AccountId) -> Result<(), Error<T>> {
        <MainAccounts<T>>::try_mutate(
            main.clone(),
            |ref mut linked_account: &mut LinkedAccount| {
                let index = linked_account
                    .proxies
                    .iter()
                    .position(|x| {
                        <AccountId as AsRef<[u8]>>::as_ref(x) == proxy.encode().as_slice()
                    })
                    .unwrap();
                linked_account.proxies.remove(index);
                Ok(())
            },
        )
    }

    pub fn get_account() -> T::AccountId {
        T::OcexId::get().into_account()
    }

    pub fn get_genesis_acc() -> T::AccountId {
        T::GenesisAccount::get().into_account()
    }

    pub fn register_acc(sender: T::AccountId) -> Result<(), Error<T>> {
        let last_account: T::AccountId = <LastAccount<T>>::get();
        <MainAccounts<T>>::try_mutate(last_account.clone(), |ref mut last_linked_account| {
            let new_linked_account: LinkedAccount = LinkedAccount::from(
                AccountId::new(
                    last_account
                        .encode()
                        .as_slice()
                        .try_into()
                        .map_err(|_| Error::AccountIdConversionFailed)?,
                ),
                AccountId::new(
                    sender
                        .clone()
                        .encode()
                        .as_slice()
                        .try_into()
                        .map_err(|_| Error::AccountIdConversionFailed)?,
                ),
            );
            <MainAccounts<T>>::insert(&sender, new_linked_account);
            <LastAccount<T>>::put(&sender);
            last_linked_account.next = Some(AccountId::new(
                sender
                    .encode()
                    .as_slice()
                    .try_into()
                    .map_err(|_| Error::AccountIdConversionFailed)?,
            ));
            Ok(())
        })
    }
}
