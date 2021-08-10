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

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{EnsureOrigin, Get},
};
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::arithmetic::{CheckedAdd, CheckedSub};
use orml_traits::{
    BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
};
use sp_runtime::traits::{Hash, Saturating, Zero};
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub(crate) type BalanceOf<T> = <T as orml_tokens::Config>::Balance;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type TreasuryAccountId: Get<Self::AccountId>;
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

    type NativeCurrency: BasicCurrencyExtended<Self::AccountId, Balance = BalanceOf<Self>>
        + BasicLockableCurrency<Self::AccountId, Balance = BalanceOf<Self>>
        + BasicReservableCurrency<Self::AccountId, Balance = BalanceOf<Self>>;
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetMetadata {
    pub name: Vec<u8>,
    pub website: Vec<u8>,
    pub team: Vec<u8>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetInfo<T: Config> {
    pub creator: T::AccountId,
    pub is_mintable: Option<T::AccountId>,
    pub is_burnable: Option<T::AccountId>,
    pub metadata: Option<AssetMetadata>,
    pub is_verified: bool,
}

impl<T: Config> Default for AssetInfo<T> {
    fn default() -> Self {
        AssetInfo {
            creator: T::AccountId::default(),
            is_mintable: None,
            is_burnable: None,
            metadata: None,
            is_verified: false,
        }
    }
}

impl<T: Config> AssetInfo<T> {
    fn from(
        creator: T::AccountId,
        is_mintable: Option<T::AccountId>,
        is_burnable: Option<T::AccountId>,
        metadata: Option<AssetMetadata>,
        is_verified: bool,
    ) -> Self {
        AssetInfo {
            creator,
            is_mintable,
            is_burnable,
            metadata,
            is_verified,
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct VestingInfo<T: Config> {
    pub amount: T::Balance,
    pub rate: T::Balance,
    pub block_no: T::BlockNumber,
}

impl<T: Config> Default for VestingInfo<T> {
    fn default() -> Self {
        VestingInfo {
            amount: T::Balance::default(),
            rate: T::Balance::default(),
            block_no: T::BlockNumber::default(),
        }
    }
}

impl<T: Config> VestingInfo<T> {
    fn from(amount: T::Balance, rate: T::Balance, block_no: T::BlockNumber) -> Self {
        VestingInfo {
            amount,
            rate,
            block_no,
        }
    }
}

pub type VestingIndex = u32;

decl_storage! {
    trait Store for Module<T: Config> as PolkadexFungible {
        /// Stores AssetInfo
        InfoAsset get(fn get_assetinfo): map hasher(identity) T::CurrencyId => AssetInfo<T>;
        InfoVesting get(fn get_vestinginfo): double_map hasher(identity) (T::AccountId,T::CurrencyId), hasher(identity) T::Hash  => VestingInfo<T>;
        FixedPDXAmount get(fn get_amount): T::Balance;
    }
}

decl_event! {
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
        <T as orml_tokens::Config>::CurrencyId,
        <T as orml_tokens::Config>::Balance
    {
        TokenIssued(CurrencyId, AccountId, Balance),
        MetadataAdded(CurrencyId, AccountId),
        AmountMinted(CurrencyId, AccountId, Balance),
        TokenVerified(CurrencyId),
        AmountBurnt(CurrencyId, AccountId, Balance),
        TokenDepositModified(Balance),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        AssetIdAlreadyExists,
        AssetIdNotExists,
        VestingInfoExists,
        NoPermissionToMint,
        NoPermissionToBurn,
        Underflow,
        Overflow,
        NotTheOwner,
        Overlimit,
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        /// Creates new Token and stores information related to that.
        ///
        /// # Parameters
        ///
        /// * `asset_id`: New Asset Id to be registered
        /// * `max_supply`: Maximum supply of new Asset Id
        /// * `mint_account`: Account which can mint amount for given Asset id
        /// * `burn_account`: Account which can burn amount for given Asset id
        /// * `existenial_deposit`: Existential Deposit
        #[weight = 10000]
        pub fn create_token(origin,
                        asset_id: T::CurrencyId,
                        max_supply: T::Balance,
                        mint_account: Option<T::AccountId>,
                        burn_account: Option<T::AccountId>,
                        _existenial_deposit: T::Balance) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(!orml_tokens::TotalIssuance::<T>::contains_key(asset_id), Error::<T>::AssetIdAlreadyExists);
            ensure!(!<InfoAsset<T>>::contains_key(&asset_id), Error::<T>::AssetIdAlreadyExists);
            let tresury_account = T::TreasuryAccountId::get();
            let amout_to_trasfer: T::Balance = FixedPDXAmount::<T>::get();
            T::NativeCurrency::transfer(&who, &tresury_account, amout_to_trasfer)?;
            let asset_info = AssetInfo::from(who.clone(), mint_account, burn_account, None, false);
            <InfoAsset<T>>::insert(asset_id, asset_info);
            orml_tokens::TotalIssuance::<T>::insert(asset_id, max_supply);
            let account_data = orml_tokens::AccountData{free: max_supply, reserved: T::Balance::zero(), frozen: T::Balance::zero()};
            orml_tokens::Accounts::<T>::insert(who.clone(), asset_id, account_data);
            Self::deposit_event(RawEvent::TokenIssued(asset_id, who, max_supply));
            Ok(())
        }

        /// Set Vesting information related to given Asset Id,
        /// Only creator of given Asset Id can set Vesting information.
        ///
        /// # Parameters
        ///
        /// * `amount`: Total amount which is going to be transferred to given Account, over fixed period of time
        /// * `asset_id`: Asset Id for which creator wants to set Vesting Info
        /// * `rate`: Rate at which transfer of amount will take place
        /// * `account`: Destination Account
        #[weight = 10000]
        pub fn set_vesting_info(origin, amount: T::Balance, asset_id: T::CurrencyId, rate: T::Balance, account: T::AccountId) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            let asset_info: AssetInfo<T> = <InfoAsset<T>>::get(asset_id);
            ensure!(asset_info.creator == who, Error::<T>::AssetIdAlreadyExists);
            let current_block_no = <system::Pallet<T>>::block_number();
            let vesting_info = VestingInfo::from(amount, rate, current_block_no);
            // Random Pallet
            let identifier = (&current_block_no, &vesting_info).using_encoded(T::Hashing::hash);
            <InfoVesting<T>>::insert((account, asset_id), identifier, vesting_info);
            Ok(())
        }

        /// Claim Vesting amount, set by given Asset Id's creator.
        ///
        /// # Parameters
        ///
        /// * `identifier`: Usual identifier which helps to find Vestion info of Given Asset Id
        /// * `asset_id`: Asset Id for which creator wants to set Vesting Info
        #[weight = 10000]
        pub fn claim_vesting(origin, identifier: T::Hash, asset_id: T::CurrencyId) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            let current_block_no = <system::Pallet<T>>::block_number();
            InfoVesting::<T>::try_mutate((who.clone(), asset_id), identifier, |ref mut vesting| {
                let block_diff = current_block_no - vesting.block_no;
                let amount = Self::block_to_balance(block_diff) * vesting.rate;
                let amount_to_be_released = if amount > vesting.amount {vesting.amount} else {amount};
                vesting.amount = vesting.amount.saturating_sub(amount_to_be_released);

                orml_tokens::Accounts::<T>::mutate(who, &asset_id, |account_data| {
                    account_data.free = account_data.free.saturating_add(amount_to_be_released);
                });
                Ok(())
            })
        }

        /// Set Metadata of given Asset Id,
        /// Only creator of given Asset Id can access this Disptachable function.
        ///
        /// # Parameters
        ///
        /// * `asset_id`: Asset Id for which creator wants to set Metadata
        /// * `metadata`: Metadata to be set for given Asset Id
        #[weight = 10000]
        pub fn set_metadata_fungible(origin, asset_id: T::CurrencyId, metadata: AssetMetadata) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoAsset<T>>::contains_key(&asset_id), <Error<T>>::AssetIdNotExists);
            let creator: AssetInfo<T> = Self::get_assetinfo(asset_id);
            ensure!(who == creator.creator, <Error<T>>::NotTheOwner);
            InfoAsset::<T>::try_mutate(&asset_id, |ref mut asset_info| {
                ensure!(metadata.name.len() <=1024 && metadata.website.len() <=1024 && metadata.team.len() <=1024, <Error<T>>::Overlimit);
                asset_info.metadata = Some(metadata);
                Self::deposit_event(RawEvent::MetadataAdded(asset_id, who));
                Ok(())
            })
        }

        /// Mints amount for given Asset Id,
        /// Account which has Minting Authority for given Asset Id can access this Dispatchable
        /// function.
        ///
        /// # Parameters
        ///
        /// * `to`: Destination account to which minted amount is going to be transferred
        /// * `asset_id`: Asset Id
        /// * `amount`: Amount which is going to be minted for given Asset Id
        #[weight = 10000]
        pub fn mint_fungible(origin, to: T::AccountId, asset_id: T::CurrencyId, amount: T::Balance) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            Self::mint_token(&who, &to,asset_id, amount)?;
            Self::deposit_event(RawEvent::AmountMinted(asset_id, who, amount));
            Ok(())
        }

        /// Burn amount for given Asset Id,
        /// Account which has Burning Authority for given Asset Id can access this Dispatchable
        /// function.
        ///
        /// # Parameters
        ///
        /// * `asset_id`: Asset Id
        /// * `amount`: Amount which is going to be burned for given Asset Id
        #[weight = 10000]
        pub fn burn_fungible(origin, asset_id: T::CurrencyId, amount: T::Balance) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            Self::burn_token(&who,asset_id, amount)?;
            Self::deposit_event(RawEvent::AmountBurnt(asset_id, who, amount));
            Ok(())
        }

        /// Verifies given Asset Id,
        /// Account which has Governance Privilege, can access this Dispatchable function.
        ///
        /// # Parameters
        ///
        /// * `asset_id`: Asset Id to be Verified
        #[weight = 10000]
        pub fn attest_token(origin, asset_id: T::CurrencyId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(<InfoAsset<T>>::contains_key(&asset_id), <Error<T>>::AssetIdNotExists);
            InfoAsset::<T>::try_mutate(&asset_id, |ref mut asset_info| {
                asset_info.is_verified = true;
                Self::deposit_event(RawEvent::TokenVerified(asset_id));
                Ok(())
            })
        }

        /// Modifies token deposit amount,
        /// Account which has Governance Privilege, can access this Dispatchable function.
        ///
        /// # Parameters
        ///
        /// * `pdx_amount`: New Token Deposit Amount
        #[weight = 10000]
        pub fn modify_token_deposit_amount(origin, pdx_amount: T::Balance) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            <FixedPDXAmount<T>>::put::<T::Balance>(pdx_amount);
            Self::deposit_event(RawEvent::TokenDepositModified(pdx_amount));
            Ok(())
        }

    }
}

impl<T: Config> Module<T> {
    pub fn mint_token(
        who: &T::AccountId,
        to: &T::AccountId,
        asset_id: T::CurrencyId,
        amount: T::Balance,
    ) -> Result<(), Error<T>> {
        let asset_info: AssetInfo<T> = <InfoAsset<T>>::get(asset_id);
        match asset_info.is_mintable {
            Some(account) if account == *who => {
                orml_tokens::TotalIssuance::<T>::try_mutate(&asset_id, |max_supply| {
                    *max_supply = max_supply
                        .checked_add(&amount)
                        .ok_or(<Error<T>>::Overflow)?;
                    orml_tokens::Accounts::<T>::try_mutate(to, asset_id, |account| {
                        account.free = account
                            .free
                            .checked_add(&amount)
                            .ok_or(<Error<T>>::Overflow)?;
                        Ok(())
                    })
                })
            }
            Some(_) => Err(<Error<T>>::NoPermissionToMint),
            None => Err(<Error<T>>::AssetIdNotExists),
        }
    }

    pub fn burn_token(
        who: &T::AccountId,
        asset_id: T::CurrencyId,
        amount: T::Balance,
    ) -> Result<(), Error<T>> {
        let asset_info: AssetInfo<T> = <InfoAsset<T>>::get(asset_id);
        match asset_info.is_burnable {
            Some(account) if account == *who => {
                orml_tokens::TotalIssuance::<T>::try_mutate(&asset_id, |max_supply| {
                    *max_supply = max_supply
                        .checked_sub(&amount)
                        .ok_or(<Error<T>>::Underflow)?;
                    orml_tokens::Accounts::<T>::try_mutate(who, asset_id, |account| {
                        account.free = account
                            .free
                            .checked_sub(&amount)
                            .ok_or(<Error<T>>::Underflow)?;
                        Ok(())
                    })
                })
            }
            Some(_) => Err(<Error<T>>::NoPermissionToBurn),
            None => Err(<Error<T>>::AssetIdNotExists),
        }
    }

    fn block_to_balance(input: T::BlockNumber) -> T::Balance {
        T::Balance::from(input.saturated_into::<u32>())
    }
}
