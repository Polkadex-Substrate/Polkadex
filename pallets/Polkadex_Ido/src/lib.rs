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
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use polkadex_primitives::assets::AssetId;
use sp_runtime::SaturatedConversion;
use sp_runtime::traits::Saturating;
use sp_runtime::traits::CheckedDiv;
use sp_runtime::traits::Zero;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    type TreasuryAccountId: Get<Self::AccountId>;

    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId = AssetId,
        Balance = Self::Balance,
    >;
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

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct FundingRound<T: Config> {
    token_a: AssetId,
    amount: T::Balance,
    token_b: AssetId,
    vesting_per_block: T::Balance,
    start_block: T::BlockNumber,
    min_allocation: T::Balance,
    max_allocation: T::Balance,
    operator_commission: T::Balance,
    token_a_priceper_token_b: T::Balance,
    close_round_block: T::BlockNumber,
    actual_raise: T::Balance
}

impl<T: Config> Default for FundingRound<T> {
    fn default() -> Self {
        FundingRound {
            token_a: AssetId::POLKADEX,
            amount: T::Balance::default(),
            token_b: AssetId::POLKADEX,
            vesting_per_block: T::Balance::default(),
            start_block: T::BlockNumber::default(),
            min_allocation: T::Balance::default(),
            max_allocation: T::Balance::default(),
            operator_commission: T::Balance::default(),
            token_a_priceper_token_b: T::Balance::default(),
            close_round_block: T::BlockNumber::default(),
            actual_raise: Zero::zero()
        }
    }
}

impl<T: Config> FundingRound<T> {
    fn from(token_a: AssetId,
            amount: T::Balance,
            token_b: AssetId,
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
            actual_raise: Zero::zero(),
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
        LastClaimBlockInfo get(fn get_last_claim_block_info): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::BlockNumber;
        InfoClaimAmount get(fn get_claim_amount): map hasher(identity) T::AccountId => T::Balance;
        InterestedParticipants get(fn get_interested_particpants): map hasher(identity) T::Hash => Vec<T::AccountId>;
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
            if <T as Config>::Currency::total_issuance(AssetId::POLKADEX) > T::MaxSupply::get()
            {
                 orml_tokens::Accounts::<T>::mutate(who.clone(), &T::NativeCurrencyId::get(), |account_data| {
                    account_data.free = account_data.free - amount;
                });
            }
            else {
                <T as Config>::Currency::transfer(AssetId::POLKADEX, &who, &T::TreasuryAccountId::get(), amount)?;
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
            token_a: AssetId,
            amount: T::Balance,
            token_b: AssetId,
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
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let (round_id, _) = T::Randomness::random(&(Self::get_wallet_account(), current_block_no, team.clone()).encode());
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
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            ensure!(current_block_no < funding_round.close_round_block && current_block_no >= funding_round.start_block, <Error<T>>::NotAllowed);
            <WhiteListInvestors<T>>::insert(round_id, investor_address.clone(), amount);
			Self::deposit_event(RawEvent::InvestorWhitelisted(round_id, investor_address));
            Ok(())
        }

        #[weight = 10000]
        pub fn participate_in_round(origin, round_id: T::Hash, amount: T::Balance) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<WhiteListInvestors<T>>::contains_key(&round_id, &investor_address), <Error<T>>::NotWhiteListed);
            let max_amount = <WhiteListInvestors<T>>::get(round_id, investor_address.clone());
            ensure!(amount >= max_amount, Error::<T>::NotAValidAmount);
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
			<T as Config>::Currency::transfer(AssetId::POLKADEX, &investor_address, &Self::get_wallet_account(), amount)?;
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            ensure!(current_block_no < funding_round.close_round_block && current_block_no > funding_round.start_block, <Error<T>>::NotAllowed);
            let total_raise = funding_round.amount.saturating_mul(funding_round.token_a_priceper_token_b);
            let investor_share = amount.checked_div(&total_raise).unwrap_or_else(Zero::zero);
            <InvestorShareInfo<T>>::insert(round_id, investor_address.clone(), investor_share);
            <InfoFundingRound<T>>::mutate(round_id, |round_details| {
                let mut actual_raise = round_details.actual_raise;
                actual_raise += amount;
            });
			Self::deposit_event(RawEvent::ParticipatedInRound(round_id, investor_address));
            Ok(())
        }

        #[weight = 10000]
        pub fn claim_tokens(origin, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            if current_block_no >= funding_round.start_block {
                let last_claim_block: T::BlockNumber = <LastClaimBlockInfo<T>>::get(round_id, investor_address.clone());
                let investor_share = <InvestorShareInfo<T>>::get(round_id, investor_address.clone());
                if !last_claim_block.is_zero() && last_claim_block > funding_round.start_block{

                    let total_released_block: T::BlockNumber = current_block_no - funding_round.start_block;
                    let tokens_released_for_given_investor: T::Balance = Self::block_to_balance(total_released_block)
                    * funding_round.vesting_per_block * investor_share;

                    <InfoClaimAmount<T>>::insert(investor_address.clone(), tokens_released_for_given_investor);

                } else {
                    let total_released_block: T::BlockNumber = current_block_no - funding_round.start_block;
                    let tokens_released_for_given_investor: T::Balance = Self::block_to_balance(total_released_block)
                    * funding_round.vesting_per_block * investor_share;

                    <InfoClaimAmount<T>>::insert(investor_address.clone(), tokens_released_for_given_investor);
                }
                <LastClaimBlockInfo<T>>::insert(round_id.clone(), investor_address.clone(), current_block_no);
            }
			Self::deposit_event(RawEvent::TokenClaimed(round_id, investor_address));
            Ok(())
        }

        #[weight = 10000]
        pub fn show_interest_in_round(origin, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            ensure!(current_block_no < funding_round.close_round_block && current_block_no >= funding_round.start_block, <Error<T>>::NotAllowed);
            InterestedParticipants::<T>::mutate(round_id, |investors| {
                    investors.push(investor_address.clone());
                });
			Self::deposit_event(RawEvent::ShowedInterest(round_id, investor_address));
             Ok(())
        }

         #[weight = 10000]
        pub fn withdraw_raise(origin, round_id: T::Hash, beneficiary: T::AccountId) -> DispatchResult {
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            ensure!(<InfoProjectTeam<T>>::contains_key(&creator.clone()), <Error<T>>::CreaterDoesNotExist);
            let info_round_id = <InfoProjectTeam<T>>::get(&creator.clone());
            ensure!(info_round_id.eq(&round_id), <Error<T>>::NotACreater);
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            let total_raise = funding_round.amount.saturating_mul(funding_round.token_a_priceper_token_b);
            <T as Config>::Currency::transfer(AssetId::POLKADEX, &creator, &beneficiary, total_raise)?;
			Self::deposit_event(RawEvent::WithdrawRaised(round_id, creator));
            Ok(())
        }

         #[weight = 10000]
        pub fn withdraw_token(origin, round_id: T::Hash, beneficiary: T::AccountId) -> DispatchResult {
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            ensure!(<InfoProjectTeam<T>>::contains_key(&creator.clone()), <Error<T>>::CreaterDoesNotExist);
            let info_round_id = <InfoProjectTeam<T>>::get(&creator.clone());
            ensure!(info_round_id.eq(&round_id), <Error<T>>::NotACreater);
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            let total_raise = funding_round.amount.saturating_mul(funding_round.token_a_priceper_token_b);
            let remaining_token = total_raise.saturating_sub(funding_round.actual_raise);
            <T as Config>::Currency::transfer(AssetId::POLKADEX, &creator, &beneficiary, remaining_token)?;
			Self::deposit_event(RawEvent::WithdrawToken(round_id, creator));
            Ok(())
        }
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
		InvestorWhitelisted(Hash, AccountId),
		ParticipatedInRound(Hash, AccountId),
		TokenClaimed(Hash, AccountId),
		ShowedInterest(Hash, AccountId),
		WithdrawRaised(Hash, AccountId),
		WithdrawToken(Hash, AccountId),
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
        NotACreater,
        CreaterDoesNotExist,
        NotAllowed
    }
}

impl<T: Config> Module<T> {

    pub fn get_wallet_account() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    fn block_to_balance(input: T::BlockNumber) -> T::Balance {
        T::Balance::from(input.saturated_into::<u32>())
    }
}

