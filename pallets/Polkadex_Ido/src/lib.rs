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
    traits::{EnsureOrigin, Get, Randomness}, PalletId
};
use frame_system as system;
use frame_system::{ensure_signed};
use sp_std::prelude::*;
use sp_runtime::traits::AccountIdConversion;
use orml_traits::{
    BasicCurrency, BasicCurrencyExtended, BasicLockableCurrency, BasicReservableCurrency,
};
use sp_runtime::traits::Saturating;
use sp_runtime::traits::CheckedDiv;
use sp_runtime::traits::Zero;

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
    type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
    type ModuleId: Get<PalletId>;
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct FundingRound<T: Config> {
    token_a: T::CurrencyId,
    amount: T::Balance,
    token_b: T::CurrencyId,
    vesting_per_block: T::Balance,
    start_block: T::BlockNumber,
    min_allocation: T::Balance,
    max_allocation: T::Balance,
    operator_commission: T::Balance,
    token_a_priceper_token_b: T::Balance,
    close_round_block: T::BlockNumber
}

impl<T: Config> Default for FundingRound<T> {
    fn default() -> Self {
        FundingRound {
            token_a: T::NativeCurrencyId::get(),
            amount: T::Balance::default(),
            token_b: T::NativeCurrencyId::get(),
            vesting_per_block: T::Balance::default(),
            start_block: T::BlockNumber::default(),
            min_allocation: T::Balance::default(),
            max_allocation: T::Balance::default(),
            operator_commission: T::Balance::default(),
            token_a_priceper_token_b: T::Balance::default(),
            close_round_block: T::BlockNumber::default()
        }
    }
}

impl<T: Config> FundingRound<T> {
    fn from(token_a: T::CurrencyId,
            amount: T::Balance,
            token_b: T::CurrencyId,
            vesting_per_block: T::Balance,
            start_block: T::BlockNumber,
            min_allocation: T::Balance,
            max_allocation: T::Balance,
            operator_commission: T::Balance,
            token_a_priceper_token_b: T::Balance,
            close_round_block: T::BlockNumber) -> Self {
        FundingRound{
            token_a,
            amount,
            token_b,
            vesting_per_block,
            start_block,
            min_allocation,
            max_allocation,
            operator_commission,
            token_a_priceper_token_b,
            close_round_block,
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as PolkadexIdo {
        InfoInvestor get(fn get_investorinfo): map hasher(identity) T::AccountId => InvestorInfo;
        InfoProjectTeam get(fn get_team): map hasher(identity) T::AccountId => T::Hash;
        InfoFundingRound get(fn get_funding_round): map hasher(identity) T::Hash => FundingRound<T>;
        WhiteListInvestors get(fn get_whitelist_investors): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::Balance;
        InvestorShareInfo get(fn get_investor_share_info): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::Balance;
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
        <T as system::Config>::Hash,
    {
        InvestorRegistered(AccountId),
        InvestorAttested(AccountId),
        FundingRoundRegistered(Hash),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        InvestorAlreadyRegistered,
        InvestorDoesNotExist,
        FundingRoundDoesNotExist,
        InvestorNotAssociatedWithFundingRound,
        FundingRoundDoesNotBelong,
        NotWhiteListed,
        NotAValidAmount,
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
        #[weight = 10000]
        pub fn register_round(
            origin,
            token_a: T::CurrencyId,
            amount: T::Balance,
            token_b: T::CurrencyId,
            vesting_per_block: T::Balance,
            start_block: T::BlockNumber,
            min_allocation: T::Balance,
            max_allocation: T::Balance,
            operator_commission: T::Balance,
            token_a_priceper_token_b: T::Balance,
            close_round_block: T::BlockNumber
        ) -> DispatchResult {
            let team: T::AccountId = ensure_signed(origin)?;
            let funding_round: FundingRound<T> = FundingRound::from(
                token_a,
                amount,
                token_b,
                vesting_per_block,
                start_block,
                min_allocation,
                max_allocation,
                operator_commission,
                token_a_priceper_token_b,
                close_round_block,
            );
            let phrase = b"polkadex_funding_round";
            let (round_id, _) = T::Randomness::random(phrase);
            <InfoFundingRound<T>>::insert(round_id, funding_round);
            <InfoProjectTeam<T>>::insert(team, round_id);
            Self::deposit_event(RawEvent::FundingRoundRegistered(round_id));
            Ok(())
        }

        #[weight = 10000]
        pub fn whitelist_investor(origin, round_id: T::Hash, investor_address: T::AccountId, amount: T::Balance) -> DispatchResult {
            let team: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            ensure!(<InfoProjectTeam<T>>::get(team).eq(&round_id), <Error<T>>::FundingRoundDoesNotBelong);
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            <WhiteListInvestors<T>>::insert(round_id, investor_address, amount);
            Ok(())
        }

        #[weight = 10000]
        pub fn participate_in_round(origin, round_id: T::Hash, amount: T::Balance) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<WhiteListInvestors<T>>::contains_key(&round_id, &investor_address), <Error<T>>::NotWhiteListed);
            let max_amount = <WhiteListInvestors<T>>::get(round_id, investor_address.clone());
            ensure!(amount >= max_amount, Error::<T>::NotAValidAmount);
            T::NativeCurrency::transfer(&investor_address, &Self::get_wallet_account(), amount)?;
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            let total_raise = funding_round.amount.saturating_mul(funding_round.token_a_priceper_token_b);
            let investor_share = amount.checked_div(&total_raise).unwrap_or_else(Zero::zero);
            <InvestorShareInfo<T>>::insert(round_id, investor_address, investor_share);
            Ok(())
        }
    }
}

impl<T: Config> Module<T> {

    pub fn get_wallet_account() -> T::AccountId {
        T::ModuleId::get().into_account()
    }
}

