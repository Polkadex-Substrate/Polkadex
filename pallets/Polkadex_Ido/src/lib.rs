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

use codec::Codec;
use codec::{Decode, Encode};
use frame_support::pallet_prelude::*;
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    log,
    traits::{EnsureOrigin, Get, Randomness},
    PalletId,
};
use frame_system as system;
use frame_system::ensure_signed;
use orml_traits::{MultiCurrency, MultiCurrencyExtended, MultiReservableCurrency};
use polkadex_primitives::assets::AssetId;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use sp_core::H256;
use sp_runtime::traits::AccountIdConversion;
use sp_runtime::traits::CheckedDiv;
use sp_runtime::traits::Saturating;
use sp_runtime::traits::Zero;
use sp_runtime::{SaturatedConversion, Permill};
use sp_std::collections::btree_map::BTreeMap;
use sp_std::collections::btree_set::BTreeSet;
use sp_std::prelude::*;
use sp_std::cmp::{max, min};
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

use pallet_polkadex_ido_primitives::{FundingRoundWithPrimitives, VoteStat};
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
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;
    /// The treasury mechanism.
    type TreasuryAccountId: Get<Self::AccountId>;
    /// The currency mechanism.
    type Currency: MultiCurrencyExtended<
        Self::AccountId,
        CurrencyId = AssetId,
        Balance = Self::Balance,
    > + MultiReservableCurrency<Self::AccountId, CurrencyId = AssetId, Balance = Self::Balance>;
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
    /// Default voting period
    type DefaultVotingPeriod : Get<Self::BlockNumber>;
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
    creator: T::AccountId,
    amount: T::Balance,
    token_b: AssetId,
    project_info_cid : Vec<u8>,
    vote_end_block : T::BlockNumber,
    vesting_end_block : T::BlockNumber,
    vesting_per_block: T::Balance,
    start_block: T::BlockNumber,
    min_allocation: T::Balance,
    max_allocation: T::Balance,
    token_a_priceper_token_b: T::Balance,
    close_round_block: T::BlockNumber,
    actual_raise: T::Balance,
}

impl<T: Config> Default for FundingRound<T> {
    fn default() -> Self {
        FundingRound {
            token_a: AssetId::POLKADEX,
            creator: T::AccountId::default(),
            amount: T::Balance::default(),
            token_b: AssetId::POLKADEX,
            project_info_cid:Vec::new(),
            vote_end_block: T::BlockNumber::default(),
            vesting_end_block: T::BlockNumber::default(),
            vesting_per_block: T::Balance::default(),
            start_block: T::BlockNumber::default(),
            min_allocation: T::Balance::default(),
            max_allocation: T::Balance::default(),
            token_a_priceper_token_b: 1_u128.saturated_into(),
            close_round_block: T::BlockNumber::default(),
            actual_raise: Zero::zero(),
        }
    }
}

impl<T: Config> FundingRound<T> {
    fn from(
        cid : Vec<u8>,
        token_a: AssetId,
        creator: T::AccountId,
        amount: T::Balance,
        token_b: AssetId,
        vote_end_block : T::BlockNumber,
        vesting_end_block: T::BlockNumber,
        vesting_per_block: T::Balance,
        start_block: T::BlockNumber,
        min_allocation: T::Balance,
        max_allocation: T::Balance,
        token_a_priceper_token_b: T::Balance,
        close_round_block: T::BlockNumber,
    ) -> Self {

        FundingRound {
            token_a,
            creator,
            amount,
            token_b,
            project_info_cid: cid,
            vote_end_block,
            vesting_end_block,
            vesting_per_block,
            start_block,
            min_allocation,
            max_allocation,
            token_a_priceper_token_b,
            close_round_block,
            actual_raise: Zero::zero(),
        }
    }

    fn to_primitive(&self) -> FundingRoundWithPrimitives<T::AccountId> {
        FundingRoundWithPrimitives {
            token_a: self.token_a,
            creator: self.creator.clone(),
            amount: self.amount.saturated_into(),
            token_b: self.token_b,
            vesting_per_block: self.vesting_per_block.saturated_into(),
            start_block: self.start_block.saturated_into(),
            vote_end_block: self.vote_end_block.saturated_into(),
            vesting_end_block: self.vesting_end_block.saturated_into(),
            project_info_cid: self.project_info_cid.clone(),
            min_allocation: self.min_allocation.saturated_into(),
            max_allocation: self.max_allocation.saturated_into(),
            token_a_priceper_token_b: self.token_a_priceper_token_b.saturated_into(),
            close_round_block: self.close_round_block.saturated_into(),
            actual_raise: self.actual_raise.saturated_into(),
        }
    }
}

#[derive(Decode, Encode, Clone)]
pub struct InterestedInvestorInfo<T: Config + frame_system::Config> {
    account_id: T::AccountId,
    amount: T::Balance,
}

#[derive(Decode, Encode, Clone)]
pub struct Voter<T: Config> {
    pub account_id : T::AccountId,
    pub votes: T::Balance,
}

#[derive(Decode, Encode, Clone)]
pub struct VoteCast<T: Config> {
    pub amount : T::Balance,
    pub unlocking_block: T::BlockNumber,
    pub voter_account: T::AccountId,
}


#[derive(Decode, Encode, Clone)]
pub struct Votes<T: Config> {
    ayes: Vec<Voter<T>>,
    nays: Vec<Voter<T>>,
}

impl<T: Config> Default for Votes<T> {
    fn default() -> Self {
        Votes {
            ayes: Vec::new(),
            nays: Vec::new(),
        }
    }
}

decl_module! {
    pub struct Module<T: Config> for enum Call where
    origin: T::Origin {

        type Error = Error<T>;

        fn deposit_event() = default;

        fn on_initialize(block_number: T::BlockNumber) -> Weight {

            let call_weight: Weight = T::DbWeight::get().reads_writes(1, 1);
            <BallotReserve<T>>::mutate(|ballot_reserve| {
                let mut garbage = Vec::new();
                for (index,reserve) in ballot_reserve.iter().enumerate() {
                    if reserve.unlocking_block == block_number {
                        T::Currency::unreserve(AssetId::POLKADEX, &reserve.voter_account, reserve.amount);
                        garbage.push(index);
                        Self::deposit_event(RawEvent::VoteAmountUnReserved(reserve.voter_account.clone(), reserve.amount));
                    }
                }
                for idx in garbage {
                    ballot_reserve.remove(idx);
                }
            });
            // Clean up WhiteListInvestors and InterestedParticipants in all expired rounds
            for (round_id,funding_round) in <InfoFundingRound<T>>::iter() {
                if block_number >= funding_round.vote_end_block {
                    let voting = <RoundVotes<T>>::get(&round_id);
                    let yes : T::Balance = voting.ayes.iter().map(|a| a.votes).fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                    let no : T::Balance = voting.nays.iter().map(|a| a.votes).fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                    if yes > no {
                        <WhitelistInfoFundingRound<T>>::insert(round_id.clone(), funding_round);
                        <InfoFundingRound<T>>::remove(&round_id);
                    }else {
                        <InfoFundingRound<T>>::remove(&round_id);
                        Self::deposit_event(RawEvent::CleanedupExpiredRound(round_id));
                    }
                }
            }

            for (round_id,funding_round) in <WhitelistInfoFundingRound<T>>::iter() {
                if block_number >= funding_round.close_round_block && !<InfoFundingRoundEnded<T>>::contains_key(round_id) {
                    let mut funding_round = funding_round.clone();
                    for (investor_address, amount) in <InterestedParticipants<T>>::iter_prefix(round_id) {
                            <WhiteListInvestors<T>>::insert(round_id, investor_address.clone(), amount);
                            let total_raise = funding_round.amount.saturating_mul(funding_round.token_a_priceper_token_b);
                            let investor_share = Permill::from_rational_approximation(amount,total_raise);
                            let round_account_id = Self::round_account_id(round_id.clone());

                            match <T as Config>::Currency::transfer(funding_round.token_b, &investor_address, &round_account_id, amount) {
                                Ok(_) => {
                                     <InvestorShareInfo<T>>::insert(round_id, investor_address.clone(), investor_share);
                                    funding_round.actual_raise = funding_round.actual_raise.saturating_add(amount);
                                    Self::deposit_event(RawEvent::ParticipatedInRound(round_id, investor_address));
                                }
                                Err(error) => {
                                    Self::deposit_event(RawEvent::ParticipatedInRoundFailed(round_id, investor_address,  error));
                                }
                            }
                    }
                    <WhitelistInfoFundingRound<T>>::insert(round_id.clone(), funding_round);
                    <InfoFundingRoundEnded<T>>::insert(round_id, true);
                }
            }
            return call_weight
        }


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
            cid : Vec<u8>,
            token_a: Option<AssetId>,
            amount: T::Balance,
            token_b: AssetId,
            vesting_per_block: T::Balance,
            funding_period: T::BlockNumber,
            min_allocation: T::Balance,
            max_allocation: T::Balance,
            token_a_priceper_token_b: T::Balance,
        ) -> DispatchResult {

            let team: T::AccountId = ensure_signed(origin)?;

            let token_a = if let Some(token_a) = token_a {
               token_a
            }else {
                 Self::create_random_token()?
            };

            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let vote_end_block = match <VotingPeriod<T>>::try_get() {
                Ok(voting_period ) => voting_period.saturating_add(current_block_no),
                Err(_) => T::DefaultVotingPeriod::get().saturating_add(current_block_no)
            };
            ensure!(token_a.ne(&token_b), <Error<T>>::TokenAEqTokenB);

            let start_block = vote_end_block.clone().saturating_add(1_u128.saturated_into());
            let close_round_block = vote_end_block.saturating_add(funding_period);

            // CID len must be less than or equal to 100
            ensure!(cid.len() <= 100, <Error<T>>::CidReachedMaxSize);
            ensure!(token_a_priceper_token_b > 0_u128.saturated_into(), <Error<T>>::PricePerTokenCantBeZero);
            ensure!(min_allocation <= max_allocation, <Error<T>>::MinAllocationMustBeEqualOrLessThanMaxAllocation);
            ensure!(start_block < close_round_block, <Error<T>>::StartBlockMustBeLessThanEndblock);
            ensure!(vote_end_block < start_block, <Error<T>>::StartBlockMustBeGreaterThanVotingPeriod);
            ensure!(vesting_per_block > Zero::zero(), <Error<T>>::VestingPerBlockMustGreaterThanZero);


            // Mint random token if user selects none: TODO: Remove in production, only for beta testes
             Self::mint_token(&team, token_a.clone(), amount)?;

            let vesting_period : u32 = (amount / vesting_per_block ).saturated_into();
            let vesting_period : T::BlockNumber = vesting_period.saturated_into();
            let vesting_end_block : T::BlockNumber = vesting_period.saturating_add(close_round_block);

            let funding_round: FundingRound<T> = FundingRound::from(
                cid,
                token_a,
                team.clone(),
                amount,
                token_b,
                vote_end_block,
                vesting_end_block,
                vesting_per_block,
                start_block,
                min_allocation,
                max_allocation,
                token_a_priceper_token_b,
                close_round_block,
            );
            let (round_id, _) = T::Randomness::random(&(Self::get_wallet_account(), current_block_no, team.clone(), Self::incr_nonce()).encode());
            let round_account_id = Self::round_account_id(round_id.clone());

            // Transfers tokens to be released to investors from team account to round account
            // This ensure that the creator has the tokens they are raising funds for
            ensure!(T::Currency::transfer(token_a, &team, &round_account_id, amount ).is_ok(), <Error<T>>::TransferTokenAFromTeamAccountFailed);
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
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id);
            ensure!(team.eq(&funding_round.creator), <Error<T>>::NotACreater);
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block_no < funding_round.close_round_block && current_block_no >= funding_round.start_block, <Error<T>>::NotAllowed);
            <WhiteListInvestors<T>>::insert(round_id, investor_address.clone(), amount);
            Self::deposit_event(RawEvent::InvestorWhitelisted(round_id, investor_address));
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
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            // Investor can only withdraw after the funding round is closed
            let round_account_id = Self::round_account_id(round_id.clone());
            let investor_share = <InvestorShareInfo<T>>::get(round_id, investor_address.clone());
            // ensure the claiming block number falls with in the vesting period
            let claim_block : T::BlockNumber = min(current_block_no, funding_round.vesting_end_block);
            let total_released_block: T::BlockNumber = claim_block - funding_round.close_round_block;
            // total_tokens_released_for_given_investor is the total available tokens for their investment
            // relative to the current block
            let total_tokens_released_for_given_investor: T::Balance = investor_share.mul_floor(Self::block_to_balance(total_released_block)
                .saturating_mul(funding_round.vesting_per_block));

            //Check if investor previously claimed the tokens
            let claimed_tokens = if <InfoClaimAmount<T>>::contains_key(&round_id, &investor_address) {
                <InfoClaimAmount<T>>::get(&round_id,&investor_address)
                }else {
                    Zero::zero()
                };
            // claimable_tokens : is the total amount of token the investor can withdraw(claim)  in their account
            let claimable_tokens = total_tokens_released_for_given_investor.saturating_sub(claimed_tokens);
            T::Currency::transfer(funding_round.token_a, &round_account_id, &investor_address, claimable_tokens);

            <InfoClaimAmount<T>>::insert(round_id, investor_address.clone(), total_tokens_released_for_given_investor);
            // TODO : remove
            <LastClaimBlockInfo<T>>::insert(round_id, investor_address.clone(), current_block_no);
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
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            ensure!(!<InterestedParticipants<T>>::contains_key(&round_id,&investor_address), Error::<T>::InvestorAlreadyShownInterest);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id);

            //Check If investor can invest amount
            ensure!(T::Currency::ensure_can_withdraw(funding_round.token_b,&investor_address, amount).is_ok(), Error::<T>::BalanceInsufficientForInteresetedAmount);

            //Ensure investment amount doesn't exceed max_allocation
            ensure!(amount <= funding_round.max_allocation && amount >= funding_round.min_allocation, Error::<T>::NotAValidAmount);

            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block_no >= funding_round.start_block && current_block_no < funding_round.close_round_block, <Error<T>>::NotAllowed);

            InterestedParticipantsAmounts::<T>::mutate(round_id, |interested_participants| {
                let total_potential_raise : T::Balance = interested_participants.iter()
                    .map(|(amount, investor)| {*amount * ( investor.len() as u128).saturated_into() })
                    .fold(T::Balance::default(), |sum, amount| {
                        sum.saturating_add(amount)
                    });
                if total_potential_raise >= funding_round.amount {
                    let participants = interested_participants.clone();
                    let replaceable_participants : Vec<(&T::AccountId,&T::Balance)> = participants.range(..=amount.clone()).flat_map(|(amount, investors)| {
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
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id);
            ensure!(creator.eq(&funding_round.creator), <Error<T>>::NotACreater);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            let round_account_id = Self::round_account_id(round_id.clone());
            ensure!(<T as Config>::Currency::transfer(funding_round.token_b, &round_account_id, &beneficiary, funding_round.actual_raise).is_ok(), Error::<T>::FundRaisedRedrawn);
            Self::deposit_event(RawEvent::WithdrawRaised(round_id, creator));
            Ok(())
        }

        #[weight = 10000]
        pub fn vote(origin, round_id: T::Hash, amount: T::Balance, vote_multiplier: u8, approve : bool) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(vote_multiplier <=  6,  Error::<T>::PeriodError);
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <InfoFundingRound<T>>::get(&round_id);
            ensure!(current_block_no < funding_round.vote_end_block , Error::<T>::VotingEnded);
            let mut voting = <RoundVotes<T>>::get(&round_id);
            let position_yes = voting.ayes.iter().position(|a| a.account_id == who);
            let position_no = voting.nays.iter().position(|a| a.account_id == who);
            // Detects first vote of the member in the motion
            let is_account_voting_first_time = position_yes.is_none() && position_no.is_none();

            //Reserves the vote amount will be later returned to user at vote.unlocking_block
            ensure!(T::Currency::reserve(AssetId::POLKADEX, &who, amount).is_ok(),Error::<T>::FailedToMoveBalanceToReserve);
            let unlocking_block = Self::vote_multiplier_to_block_number(vote_multiplier);
            let voter = Voter{
                account_id : who.clone(),
                votes: max(amount, amount.saturating_mul(vote_multiplier.saturated_into())),
            };
            let vote_cast = VoteCast {
                amount : amount.clone(),
                unlocking_block,
                voter_account: who.clone()
            };
            <BallotReserve<T>>::mutate(|reserve|{
                reserve.push(vote_cast);
            });
            if approve {
                if position_yes.is_none() {
                    voting.ayes.push(voter);
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_no {
                    voting.nays.swap_remove(pos);
                }
            } else {
                if position_no.is_none() {
                    voting.nays.push(voter);
                } else {
                    Err(Error::<T>::DuplicateVote)?
                }
                if let Some(pos) = position_yes {
                    voting.ayes.swap_remove(pos);
                }
            }
            <RoundVotes<T>>::insert(round_id, voting);
            Ok(())
        }

        #[weight = 10000]
        pub fn set_vote_period(origin, period : T::BlockNumber) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            <VotingPeriod<T>>::put(period);
            Ok(())
        }

        #[weight = 10000]
        pub fn approve_ido_round(origin, round_id : T::Hash) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!<WhitelistInfoFundingRound<T>>::contains_key(&round_id), <Error<T>>::RoundAlreadyApproved);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), <Error<T>>::FundingRoundDoesNotExist);
            let funding_round = <InfoFundingRound<T>>::get(&round_id);
            <WhitelistInfoFundingRound<T>>::insert(round_id.clone(), funding_round);
            <InfoFundingRound<T>>::remove(&round_id);
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
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id);
            ensure!(creator.eq(&funding_round.creator), <Error<T>>::NotACreater);
            // Check if there is any left to withdraw
            let total_tokens_bought_by_investors = funding_round.actual_raise.saturating_mul(1_u32.saturated_into::<T::Balance>()/ funding_round.token_a_priceper_token_b);
            let remaining_token = funding_round.amount.saturating_sub(total_tokens_bought_by_investors);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            ensure!(remaining_token > Zero::zero(), Error::<T>::WithdrawalBlocked);
            let round_account_id = Self::round_account_id(round_id.clone());
            //Transfers to remaining token back to creator after round.
            <T as Config>::Currency::transfer(funding_round.token_a, &round_account_id, &beneficiary, remaining_token)?;
            Self::deposit_event(RawEvent::WithdrawToken(round_id, creator));
            Ok(())
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as PolkadexIdo {
        /// A mapping of Investor and its KYC status
        InfoInvestor get(fn get_investorinfo): map hasher(identity) T::AccountId => InvestorInfo;
        /// A mapping between funding round creator and funding round id
        InfoProjectTeam get(fn get_team): map hasher(identity) T::AccountId => T::Hash;
        /// A mapping between round id and funding round information
        InfoFundingRound get(fn get_funding_round): map hasher(identity) T::Hash => FundingRound<T>;
        /// Whitelisted info funding round
        WhitelistInfoFundingRound get(fn get_whitelist_funding_round): map hasher(identity) T::Hash => FundingRound<T>;
        /// For each round, we keep mapping between whitelist investors and the amount, they will be investing
        WhiteListInvestors get(fn get_whitelist_investors): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::Balance;
        /// For each round, we keep mapping between participants and the amount, they will be using
        InvestorShareInfo get(fn get_investor_share_info): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => Permill;
        /// For each round, we keep mapping between investors and the block, when they will claim token
        LastClaimBlockInfo get(fn get_last_claim_block_info): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  => T::BlockNumber;
        /// A mapping between investor and claim amount
        InfoClaimAmount get(fn get_claim_amount): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId => T::Balance;
        /// A mapping between funding round id and its InterestedParticipants plus the amount they are willing to invest
        InterestedParticipants get(fn get_interested_particpants): double_map hasher(identity) T::Hash, hasher(identity) T::AccountId  =>  T::Balance;
        /// A mapping between funding round id and amount interested participant are will to invest
        InterestedParticipantsAmounts get(fn get_interested_particpants_amounts): map hasher(identity) T::Hash => BTreeMap<T::Balance, BTreeSet<T::AccountId>>;

        /// Only for debugging will be removed
        InfoFundingRoundEnded get(fn get_funding_round_ended): map hasher(identity) T::Hash => bool;

        Nonce get(fn nonce): u128;

        VotingPeriod get(fn get_voting_period): T::BlockNumber = T::DefaultVotingPeriod::get();

        RoundVotes get(fn get_round_votes): map hasher(identity) T::Hash => Votes<T>;

        BallotReserve get(fn get_ballot_reserve): Vec<VoteCast<T>>;
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

decl_event! {
    pub enum Event<T>
    where
        <T as system::Config>::AccountId,
        <T as system::Config>::Hash,
        <T as orml_tokens::Config>::Balance,
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
        CleanedupExpiredRound(Hash),
        RoundWhitelisted(Hash),
        VoteAmountUnReserved(AccountId,Balance),
        ParticipatedInRoundFailed(Hash, AccountId,sp_runtime::DispatchError),
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
        /// Investor already participated in a round error
        InvestorAlreadyParticipated,
        /// Investor already shown interest
        InvestorAlreadyShownInterest,
        /// Investor Account Balance doesnt match interest amount
        BalanceInsufficientForInteresetedAmount,
        /// Price Per Token Error
        PricePerTokenCantBeZero,
        /// Min allocation cant be greater than Max allocation
        MinAllocationMustBeEqualOrLessThanMaxAllocation,
        /// Failed to transfer TokenAFromTeamAccount
        TransferTokenAFromTeamAccountFailed,
        /// TokenA cannot be equal to TokenB
        TokenAEqTokenB,
        /// Block withdrawal when round is active
        WithdrawalBlocked,
        /// Block claim token when round is active,
        ClaimTokenBlocked,
        /// FundRaised already Withdrawn
        StartBlockMustBeLessThanEndblock,
        StartBlockMustBeGreaterThanVotingPeriod,
        RoundAlreadyApproved,
        FundRaisedRedrawn,
        PeriodError,
        VotingEnded,
        DuplicateVote,
        FailedToMoveBalanceToReserve,
        FundingRoundNotApproved,
        CidReachedMaxSize,
        VestingPerBlockMustGreaterThanZero,
        MintNativeTokenForbidden
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
        let (nonce, _) = current_nonce.overflowing_add(1);
        <Nonce>::put(nonce);
        <Nonce>::get()
    }

    pub fn pallet_account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    pub fn rounds_by_investor(
        account: T::AccountId,
    ) -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        <InvestorShareInfo<T>>::iter()
            .filter_map(|(round_id, investor, _)| {
                if investor != account {
                    None
                } else {
                    let round_info = <WhitelistInfoFundingRound<T>>::get(&round_id);
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect()
    }

    pub fn rounds_by_creator(
        account: T::AccountId,
    ) -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        let whitelisted_funding_round : Vec<_> = <WhitelistInfoFundingRound<T>>::iter()
            .filter_map(|(round_id,round_info)| {
                if round_info.creator != account {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let pending_funding_round : Vec<_> = <InfoFundingRound<T>>::iter()
            .filter_map(|(round_id, round_info)| {
                if round_info.creator != account {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let mut mixed_funding_rounds = Vec::new();
        mixed_funding_rounds.extend_from_slice(&whitelisted_funding_round);
        mixed_funding_rounds.extend_from_slice(&pending_funding_round);
        mixed_funding_rounds
    }

    pub fn votes_stat(
        round_id: T::Hash,
    ) -> VoteStat {
        match <RoundVotes<T>>::try_get(&round_id) {
            Ok(voting) => {
                let yes : T::Balance = voting.ayes.iter().map(|a| a.votes).fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                let no : T::Balance = voting.nays.iter().map(|a| a.votes).fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));

                VoteStat {
                    yes: yes.saturated_into(),
                    no: no.saturated_into()
                }
            }
            Err(_) => {
                VoteStat {
                    yes: 0,
                    no: 0
                }
            }
        }
    }

    pub fn vote_multiplier_to_block_number(multiplier : u8 ) -> T::BlockNumber {
        // 1 day in blocks total seconds (86400 secs) in a day divided by block time (6 secs)
        let lock_period: u32 = 28 * (86400 / 6);
        let factor = if multiplier == 0 {
            (lock_period / 10) as u32
        } else {
            lock_period * multiplier as u32
        };
        let current_block_no = <frame_system::Pallet<T>>::block_number();
        current_block_no.saturating_add(factor.saturated_into())
    }

    pub fn create_random_token() -> Result<AssetId, sp_runtime::DispatchError> {
        let seed = <T as Config>::RandomnessSource::random_seed();
        let mut rng = ChaChaRng::from_seed(*seed.0.as_fixed_bytes());
        let random_asset_id : u64 = rng.gen();
        let new_asset = AssetId::Asset(random_asset_id);
        Ok(new_asset)
    }

    pub fn mint_token(who: &T::AccountId,asset_id : AssetId, amount : T::Balance) -> DispatchResult{
        ensure!(asset_id.ne(&AssetId::POLKADEX), <Error<T>>::MintNativeTokenForbidden);
        T::Currency::deposit(asset_id, who, amount)
    }
}
