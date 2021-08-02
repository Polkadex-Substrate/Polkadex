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

//! # Polkadex IDO Pallet
//!
//! Polkadex IDO pallet helps teams and projects to raise capital for their work by
//! issuing tokens for Seed, Private and Public investment rounds.
//!
//! ### Goals
//!
//! It will take care of
//!
//! * Vesting Period
//! * 3 levels of KYC for Investors
//! * Airdrop
//! * Parachain Offering(useful for future projects raising DOT/KSM)
//! * Project Attestation by Governance
//! * Stake More tokens or stake longer for buying more tokens
//! * Set commission for token project

#![cfg_attr(not(feature = "std"), no_std)]
// Clippy warning diabled for to many arguments on line#157
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unused_unit)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{EnsureOrigin, Get, Randomness},
    PalletId,
};
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended};
use polkadex_primitives::assets::AssetId;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::CheckedDiv;
use sp_runtime::traits::Saturating;
use sp_runtime::traits::Zero;
use sp_runtime::SaturatedConversion;
use sp_std::prelude::*;
use sp_std::collections::btree_map::BTreeMap;
use sp_std::collections::btree_set::BTreeSet;
use rand::{Rng, seq::SliceRandom, SeedableRng};
use rand_chacha::ChaChaRng;
use polkadex_primitives::BlockNumber;
use sp_core::H256;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

/// The module configuration trait.
pub trait Config: system::Config + orml_tokens::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    /// The origin which may attests the investor to take part in the IDO pallet.
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    /// The treasury mechanism.
    type TreasuryAccountId: Get<Self::AccountId>;
    /// The currency mechanism.
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId=AssetId,
        Balance=Self::Balance,
    >;
    /// The native currency ID type
    type NativeCurrencyId: Get<Self::CurrencyId>;
    /// The basic amount of funds that must be reserved for an Polkadex.
    type IDOPDXAmount: Get<Self::Balance>;
    /// Maximum supply for IDO
    type MaxSupply: Get<Self::Balance>;
    /// The generator used to supply randomness to IDO
    type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
    /// Randomness Source for random participant seed
    type RandomnessSource: Randomness<H256, Self::BlockNumber>;
    /// The IDO's module id
    type ModuleId: Get<PalletId>;
    /// Weight information for extrinsics in this pallet.
    type WeightIDOInfo: WeightInfo;
}

/// The type of `KYCStatus` that provides level of KYC
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum KYCStatus {
    Tier0,
    Tier1,
    Tier2,
}

/// KYC information for an investor.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct InvestorInfo {
    /// Level of KYC status
    pub kyc_status: KYCStatus,
}

impl Default for InvestorInfo {
    fn default() -> Self {
        InvestorInfo {
            kyc_status: KYCStatus::Tier0,
        }
    }
}

/// All information for funding round
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
    actual_raise: T::Balance,
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
            actual_raise: Zero::zero(),
        }
    }
}

impl<T: Config> FundingRound<T> {
    fn from(
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
    ) -> Self {
        FundingRound {
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

#[derive(Decode, Encode, Clone)]
pub struct InterestedInvestorInfo<T: Config + frame_system::Config> {
    account_id: T::AccountId,
    amount: T::Balance,
}


decl_storage! {
    trait Store for Module<T: Config> as PolkadexIdo {
        /// A mapping of Investor and its KYC status
        InfoInvestor get(fn get_investorinfo): map hasher(identity) T::AccountId => InvestorInfo;
        /// A mapping between funding round creator and funding round id
        InfoProjectTeam get(fn get_team): map hasher(identity) T::AccountId => T::Hash;
        /// A mapping between round id and funding round information
        InfoFundingRound get(fn get_funding_round): map hasher(identity) T::Hash => FundingRound<T>;
        /// For each round, we keep mapping between whitelist investors and the amount, they will be investing
        WhiteListInvestors get(fn get_whitelist_investors): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::Balance;
        /// For each round, we keep mapping between participants and the amount, they will be using
        InvestorShareInfo get(fn get_investor_share_info): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::Balance;
        /// For each round, we keep mapping between investors and the block, when they will claim token
        LastClaimBlockInfo get(fn get_last_claim_block_info): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::BlockNumber;
        /// A mapping between investor and claim amount
        InfoClaimAmount get(fn get_claim_amount): map hasher(identity) T::AccountId => T::Balance;
        /// A mapping between funding round id and its InterestedParticipants plus the amount they are willing to invest
        InterestedParticipants get(fn get_interested_particpants): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  =>  T::Balance;
        /// A mapping between funding round id and amount interested participant are will to invest
        InterestedParticipantsAmounts get(fn get_interested_particpants_amounts): map hasher(identity) T::Hash => BTreeMap<T::Balance, BTreeSet<T::AccountId>>;

        Nonce get(fn nonce): u128;
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

        /// Registers a new investor to allow participating in funding round.
        ///
        /// # Parameters
        ///
        /// * `origin`: Account to be registered as Investor
        #[weight = T::WeightIDOInfo::register_investor()]
        pub fn register_investor(origin) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(!<InfoInvestor<T>>::contains_key(&who), Error::<T>::InvestorAlreadyRegistered);
            let amount: T::Balance = T::IDOPDXAmount::get();
            if <T as Config>::Currency::total_issuance(AssetId::POLKADEX) > T::MaxSupply::get()
            {
                 ensure!(T::Currency::withdraw(AssetId::POLKADEX,&who,amount).is_ok(),  Error::<T>::WithdrawError);
            }
            else {
                <T as Config>::Currency::transfer(AssetId::POLKADEX, &who, &T::TreasuryAccountId::get(), amount)?;
            }
            let investor_info = InvestorInfo::default();
            <InfoInvestor<T>>::insert(who.clone(), investor_info);
            Self::deposit_event(RawEvent::InvestorRegistered(who));
            Ok(())
        }

        /// Attests the investor to take part in the IDO pallet.
        /// Attestor is part of the governance committee of IDO pallet.
        ///
        /// # Parameters
        ///
        /// * `investor`: Registered investor
        /// * `kyc_status`: Level of KYC Status
        #[weight = 10000] //TODO : Connect weight file
        pub fn attest_investor(origin, investor: T::AccountId, kyc_status: KYCStatus) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor), <Error<T>>::InvestorDoesNotExist);
            InfoInvestor::<T>::try_mutate(&investor, |ref mut investor_info| {
                investor_info.kyc_status = kyc_status;
                Self::deposit_event(RawEvent::InvestorAttested(investor.clone()));
                Ok(())
            })
        }

        /// Registers a funding round with the amount as the total allocation for this round and vesting period.
        ///
        /// # Parameters
        ///
        /// * `token_a`: The Project token
        /// * `amount`: Amount for funding round
        /// * `token_b`: Token in which funding is received
        /// * `vesting_per_block`: Vesting per block
        /// * `start_block`: Start block of funding round
        /// * `min_allocation`: Minimum allocation
        /// * `max_allocation`: Maximum allocation
        /// * `operator_commission`: Commission for operrator
        /// * `token_a_priceper_token_b`: Priceper amount for project token
        /// * `close_round_block`: Closing block of funding round
        #[weight = T::WeightIDOInfo::register_round()]
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
            let (round_id, _) = T::Randomness::random(&(Self::get_wallet_account(), current_block_no, team.clone(), Self::incr_nonce()).encode());
            <InfoFundingRound<T>>::insert(round_id, funding_round);
            <InfoProjectTeam<T>>::insert(team, round_id);
            Self::deposit_event(RawEvent::FundingRoundRegistered(round_id));
            Ok(())
        }

        /// Project team whitelists investor for the given round for the given amount.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `investor_address`: Investor
        /// * `amount`: The max amount that investor will be investing in tokenB
        #[weight = T::WeightIDOInfo::whitelist_investor()]
        pub fn whitelist_investor(origin, round_id: T::Hash, investor_address: T::AccountId, amount: T::Balance) -> DispatchResult {
            let team: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            ensure!(<InfoProjectTeam<T>>::get(team).eq(&round_id), <Error<T>>::FundingRoundDoesNotBelong);
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            ensure!(current_block_no < funding_round.close_round_block && current_block_no >= funding_round.start_block, <Error<T>>::NotAllowed);
            <WhiteListInvestors<T>>::insert(round_id, investor_address.clone(), amount);
            Self::deposit_event(RawEvent::InvestorWhitelisted(round_id, investor_address));
            Ok(())
        }

        /// Stores information about whitelisted investor, participating in round.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `amount`: Amount to be transferred to wallet.
        #[weight = 10000]
        pub fn participate_in_round(origin, round_id: T::Hash, amount: T::Balance) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<WhiteListInvestors<T>>::contains_key(&round_id, &investor_address), <Error<T>>::NotWhiteListed);
            let max_amount = <WhiteListInvestors<T>>::get(round_id, investor_address.clone());
            ensure!(amount >= max_amount, Error::<T>::NotAValidAmount);
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            ensure!(current_block_no < funding_round.close_round_block && current_block_no > funding_round.start_block, <Error<T>>::NotAllowed);
            let total_raise = funding_round.amount.saturating_mul(funding_round.token_a_priceper_token_b);
            let investor_share = amount.checked_div(&total_raise).unwrap_or_else(Zero::zero);
            let round_account_id = Self::round_account_id(round_id.clone());
             <T as Config>::Currency::transfer(AssetId::POLKADEX, &investor_address, &round_account_id, amount)?;
            <InvestorShareInfo<T>>::insert(round_id, investor_address.clone(), investor_share);
            <InfoFundingRound<T>>::mutate(round_id, |round_details| {
                round_details.actual_raise += amount;
            });
            Self::deposit_event(RawEvent::ParticipatedInRound(round_id, investor_address));
            Ok(())
        }

        /// Investor claiming for a particular funding round.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[weight = T::WeightIDOInfo::claim_tokens()]
        pub fn claim_tokens(origin, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            if current_block_no >= funding_round.start_block {
                let investor_share = <InvestorShareInfo<T>>::get(round_id, investor_address.clone());
                let total_released_block: T::BlockNumber = current_block_no - funding_round.start_block;
                let tokens_released_for_given_investor: T::Balance = Self::block_to_balance(total_released_block)
                .saturating_mul(funding_round.vesting_per_block)
                .saturating_mul(investor_share);

                <InfoClaimAmount<T>>::insert(investor_address.clone(), tokens_released_for_given_investor);
                <LastClaimBlockInfo<T>>::insert(round_id, investor_address.clone(), current_block_no);
            }
            Self::deposit_event(RawEvent::TokenClaimed(round_id, investor_address));
            Ok(())
        }

        /// Stores information about investors, showing interest in funding round.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[weight = T::WeightIDOInfo::show_interest_in_round()]
        pub fn show_interest_in_round(origin, round_id: T::Hash, amount : T::Balance) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            ensure!(!<InterestedParticipants<T>>::contains_key(&round_id,&investor_address), Error::<T>::InvestorAlreadyShownInterest);
            //Check If investor can invest amount
            ensure!(T::Currency::ensure_can_withdraw(AssetId::POLKADEX,&investor_address, amount).is_ok(), Error::<T>::BalanceInsufficientForInteresetedAmount);

            let funding_round : FundingRound<T> = <InfoFundingRound<T>>::get(round_id);

            let max_amount = funding_round.max_allocation;
            //Ensure investment amount doesn't exceed max_allocation
            ensure!(amount <= max_amount, Error::<T>::NotAValidAmount);

            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block_no < funding_round.close_round_block && current_block_no >= funding_round.start_block, <Error<T>>::NotAllowed);

            InterestedParticipantsAmounts::<T>::mutate(round_id, |interested_participants| {
                let total_potential_raise : T::Balance = interested_participants.iter()
                    .map(|(amount, investor)| {*amount * ( investor.len() as u128).saturated_into() })
                    .fold(T::Balance::default(), |sum, amount| {
                        sum.saturating_add(amount)
                    });
                if total_potential_raise >= funding_round.amount {
                    let participants = interested_participants.clone();
                    let replaceable_participants : Vec<(&T::AccountId,&T::Balance)> = participants.range(..amount.clone()).flat_map(|(amount, investors)| {
                        investors.iter().map( move |investor| {
                            (investor, amount)
                        })
                    }).collect();
                    let seed = <T as Config>::RandomnessSource::random_seed();
                    let mut rng = ChaChaRng::from_seed(*seed.0.as_fixed_bytes());
                    let random_index = rng.gen_range(0..replaceable_participants.len());
                    let evicted_participant = replaceable_participants[random_index];
                    <InterestedParticipants<T>>::remove(round_id,evicted_participant.0);
                    let is_empty_participants = interested_participants.get_mut(evicted_participant.1).and_then(|investors| {
                        investors.remove(evicted_participant.0);
                        Some(investors.is_empty())
                    });

                   match is_empty_participants {
                        Some(is_empty) => {
                            if is_empty {
                                interested_participants.remove(evicted_participant.1);
                            }
                        }
                        _ => {}
                    };

                }
                <InterestedParticipants<T>>::insert(round_id,investor_address.clone(), amount.clone());
                let participants = interested_participants.entry(amount).or_insert(BTreeSet::new());
                participants.insert(investor_address.clone());
                Self::deposit_event(RawEvent::ShowedInterest(round_id, investor_address));

            });

             Ok(())
        }

        /// Transfers the raised amount to another address,
        /// only the round creator can call this or the governance.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `beneficiary`: Account Id of Beneficiary
         #[weight = 10000]
        pub fn withdraw_raise(origin, round_id: T::Hash, beneficiary: T::AccountId) -> DispatchResult {
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            ensure!(<InfoProjectTeam<T>>::contains_key(&creator), <Error<T>>::CreaterDoesNotExist);
            let info_round_id = <InfoProjectTeam<T>>::get(&creator);
            ensure!(info_round_id.eq(&round_id), <Error<T>>::NotACreater);
            let funding_round = <InfoFundingRound<T>>::get(round_id);
            let round_account_id = Self::round_account_id(round_id.clone());
            <T as Config>::Currency::transfer(AssetId::POLKADEX, &round_account_id, &beneficiary, funding_round.actual_raise)?;
            Self::deposit_event(RawEvent::WithdrawRaised(round_id, creator));
            Ok(())
        }

        /// Transfers the remaining tokens to another address,
        /// only the round creator can call this or the governance.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `beneficiary`: Account Id of Beneficiary
         #[weight = 10000]
        pub fn withdraw_token(origin, round_id: T::Hash, beneficiary: T::AccountId) -> DispatchResult {
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            ensure!(<InfoProjectTeam<T>>::contains_key(&creator), <Error<T>>::CreaterDoesNotExist);
            let info_round_id = <InfoProjectTeam<T>>::get(&creator);
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

decl_event! {
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
        <T as system::Config>::Hash,
    {
        /// Investor has been registered
        InvestorRegistered(AccountId),
        /// Investor has been attested
        InvestorAttested(AccountId),
        /// Funding round has been registered
        FundingRoundRegistered(Hash),
        /// Investor has been whitelisted
        InvestorWhitelisted(Hash, AccountId),
        /// Participant has been added
        ParticipatedInRound(Hash, AccountId),
        /// Token has been claimed
        TokenClaimed(Hash, AccountId),
        /// Showed interest in funding round
        ShowedInterest(Hash, AccountId),
        /// Transferred raised amount
        WithdrawRaised(Hash, AccountId),
        /// Transferred remaining tokens
        WithdrawToken(Hash, AccountId),
    }
}

decl_error! {
    pub enum Error for Module<T: Config> {
        /// Investor is already registered
        InvestorAlreadyRegistered,
        /// Investor does not exist
        InvestorDoesNotExist,
        /// Funding round does not exist
        FundingRoundDoesNotExist,
        /// Investor is not associated with funding round
        InvestorNotAssociatedWithFundingRound,
        /// Funding round does not belong
        FundingRoundDoesNotBelong,
        /// Investor is not whitelisted
        NotWhiteListed,
        /// Not a valid amount
        NotAValidAmount,
        /// Not a creator of funding round
        NotACreater,
        /// Creator does not exist
        CreaterDoesNotExist,
        /// Not allowed
        NotAllowed,
        /// Withdraw Error
        WithdrawError,
        /// Investor already shown interest
        InvestorAlreadyShownInterest,
        /// Investor Account Balance doesnt match interest amount
        BalanceInsufficientForInteresetedAmount
    }
}

impl<T: Config> Module<T> {
    pub fn get_wallet_account() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    fn block_to_balance(input: T::BlockNumber) -> T::Balance {
        T::Balance::from(input.saturated_into::<u32>())
    }

    pub fn round_account_id(hash: T::Hash) -> T::AccountId {
        T::ModuleId::get().into_sub_account(hash)
    }

    fn incr_nonce() -> u128 {
        let current_nonce: u128 = <Nonce>::get();
        <Nonce>::put(current_nonce.saturating_add(1));
        <Nonce>::get()
    }
}
