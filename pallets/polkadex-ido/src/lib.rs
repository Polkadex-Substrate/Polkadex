// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
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
//! - ['Config`]
//! - ['Call']
//!
//! ### Overview
//!
//! Polkadex IDO pallet helps teams and projects to raise capital for their work by
//! issuing tokens for Seed, Private and Public investment rounds.
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
//! ### Terminology
//! - ** Vote: ** A value that can either be in approval ("Aye") or rejection ("Nay") of a particular Round
//! - ** FundingRound: ** Funding round for an IDO project
//! - ** InvestorLockData: ** Investors funds lock info
//! - ** VoteCast: ** structure for storing vote amount and unlocking block for a voter/investor
//! - ** Investor: ** investor for the funding round
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! #### Public
//!
//! Investor actions:
//! - `register_investor` - registers a new investor to allow participating in funding round
//! - `investor_unlock_fund` - Unlocks investor locked fund for registering as investor
//! - `show_interest_in_round` - Stores information about investors, showing interest in funding round.
//! - `claim_tokens` - Investor claiming for a particular funding round.
//! - `vote` - Vote for funding round to be whitelisted or not
//! IDO round creator actions:
//! - `register_round` - Registers a funding round with the amount as the total allocation for this round and vesting period.
//! - `whitelist_investor` -  Project team whitelists investor for the given round for the given amount.
//! - `withdraw_raise` -  Transfers the raised amount to another address,
//! - `withdraw_token` - Transfers the remaining tokens to another address
//! Governance Actions:
//! - `set_vote_period` - Sets voting period for funding rounds
//! - `set_investor_lock_fund_period` - Sets investor fund lock period
//! - `approve_ido_round` - Force ido approval by governance
//! - `attest_investor` - Attests the investor to take part in the IDO pallet.
//!

#![cfg_attr(not(feature = "std"), no_std)]
// Clippy warning diabled for to many arguments on line#157
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unused_unit)]

use frame_support::{
    dispatch::DispatchResult,
    ensure,
    pallet_prelude::*,
    traits::{
        tokens::fungibles::{Create, Inspect, Mutate, Transfer, Unbalanced},
        EnsureOrigin, Get, Randomness, WithdrawReasons,
    },
    PalletId,
};
use frame_support::traits::tokens::fungible;
use frame_system as system;
use frame_system::ensure_signed;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaChaRng;
use scale_info::StaticTypeInfo;
use sp_core::H256;
use sp_runtime::{
    traits::{AccountIdConversion, Saturating, Zero},
    Perbill, Perquintill, SaturatedConversion,
};
use sp_std::{
    cmp::{max, min},
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    prelude::*,
};

pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

use pallet_polkadex_ido_primitives::{FundingRoundWithPrimitives, VoteStat, StringAssetId};
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;


use polkadex_primitives::assets::AssetId;
use frame_support::traits::{Currency, ReservableCurrency, ExistenceRequirement};

type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::{
        pallet_prelude::*,
        traits::tokens::fungibles::{Create, Inspect, Mutate},
        PalletId,
    };
    use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
    use sp_core::{H160, H256};
    use sp_runtime::traits::One;
    use sp_std::prelude::*;

    use super::*;


    use polkadex_primitives::assets::AssetId;
    use pallet_polkadex_ido_primitives::AccountId;

    /// The module configuration trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// The origin which may attests the investor to take part in the IDO pallet.
        type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
        /// The treasury mechanism.
        #[pallet::constant]
        type TreasuryAccountId: Get<Self::AccountId>;
        /// Balances Pallet
        type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId> + fungible::Inspect<Self::AccountId>;
        /// The basic amount of funds that must be reserved for an Polkadex.
        #[pallet::constant]
        type IDOPDXAmount: Get<BalanceOf<Self>>;
        /// Maximum supply for IDO
        #[pallet::constant]
        type MaxSupply: Get<BalanceOf<Self>>;
        /// The generator used to supply randomness to IDO
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
        /// Randomness Source for random participant seed
        type RandomnessSource: Randomness<H256, Self::BlockNumber>;
        /// The IDO's module id
        #[pallet::constant]
        type ModuleId: Get<PalletId>;
        /// Weight information for extrinsics in this pallet.
        type WeightIDOInfo: WeightInfo;
        /// Default voting period
        #[pallet::constant]
        type DefaultVotingPeriod: Get<Self::BlockNumber>;
        /// Default investor locking period
        #[pallet::constant]
        type DefaultInvestorLockPeriod: Get<Self::BlockNumber>;
        /// Minimum deposit to create PDEX account for round id
        #[pallet::constant]
        type ExistentialDeposit: Get<BalanceOf<Self>>;

        /// One PDEX amount in u128
        #[pallet::constant]
        type OnePDEX : Get<u128>;

        type AssetManager: Create<<Self as frame_system::Config>::AccountId>
        + Mutate<<Self as frame_system::Config>::AccountId, Balance=u128, AssetId=u128>
        + Inspect<<Self as frame_system::Config>::AccountId>
        + Transfer<<Self as frame_system::Config>::AccountId>
        + Unbalanced<<Self as frame_system::Config>::AccountId>;
    }

    #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct InvestorLockData<T: Config> {
        pub amount: BalanceOf<T>,
        pub unlock_block: T::BlockNumber,
    }

    impl<T: Config> InvestorLockData<T> {
        pub fn new(amount: BalanceOf<T>, unlock_block: T::BlockNumber) -> Self {
            Self { amount, unlock_block }
        }
    }

    impl<T: Config> Default for InvestorLockData<T> {
        fn default() -> Self {
            InvestorLockData { amount: Default::default(), unlock_block: Default::default() }
        }
    }

    /// The type of `KYCStatus` that provides level of KYC
    #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
    pub enum KYCStatus {
        Tier0,
        Tier1,
        Tier2,
    }

    /// KYC information for an investor.
    #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct InvestorInfo<T: Config> {
        /// Level of KYC status
        pub kyc_status: KYCStatus,
        pub lock_data: Option<InvestorLockData<T>>,
    }

    impl<T: Config> Default for InvestorInfo<T> {
        fn default() -> Self {
            InvestorInfo { kyc_status: KYCStatus::Tier0, lock_data: Default::default() }
        }
    }

    /// All information for funding round
    #[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct FundingRound<T: Config> {
        pub token_a: AssetId,
        pub creator: T::AccountId,
        pub amount: BalanceOf<T>,
        pub token_b: AssetId,
        pub project_info_cid: Vec<u8>,
        pub vesting_end_block: T::BlockNumber,
        pub vesting_per_block: BalanceOf<T>,
        pub start_block: T::BlockNumber,
        pub min_allocation: BalanceOf<T>,
        pub max_allocation: BalanceOf<T>,
        pub token_a_priceper_token_b: BalanceOf<T>,
        pub close_round_block: T::BlockNumber,
        pub actual_raise: BalanceOf<T>,
    }

    impl<T: Config> FundingRound<T> {
        fn from(
            cid: Vec<u8>,
            token_a: AssetId,
            creator: T::AccountId,
            amount: BalanceOf<T>,
            token_b: AssetId,
            vesting_end_block: T::BlockNumber,
            vesting_per_block: BalanceOf<T>,
            start_block: T::BlockNumber,
            min_allocation: BalanceOf<T>,
            max_allocation: BalanceOf<T>,
            token_a_priceper_token_b: BalanceOf<T>,
            close_round_block: T::BlockNumber,
        ) -> Self {
            FundingRound {
                token_a,
                creator,
                amount,
                token_b,
                project_info_cid: cid,
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

        pub fn to_primitive(&self) -> FundingRoundWithPrimitives<T::AccountId> {
            FundingRoundWithPrimitives {
                token_a: StringAssetId::from(self.token_a),
                creator: self.creator.clone(),
                amount: self.amount.saturated_into(),
                token_b: StringAssetId::from(self.token_b),
                vesting_per_block: self.vesting_per_block.saturated_into(),
                start_block: self.start_block.saturated_into(),
                vesting_end_block: self.vesting_end_block.saturated_into(),
                project_info_cid: self.project_info_cid.clone(),
                min_allocation: self.min_allocation.saturated_into(),
                max_allocation: self.max_allocation.saturated_into(),
                token_a_priceper_token_b: self.token_a_priceper_token_b.saturated_into(),
                close_round_block: self.close_round_block.saturated_into(),
                actual_raise: self.actual_raise.saturated_into(),
            }
        }

        pub fn token_a_price_per_1e12_token_b(&self) -> Perbill {
            let token_a_priceper_token_b: u128 = self.token_a_priceper_token_b.saturated_into();
            Perbill::from_rational(token_a_priceper_token_b, T::OnePDEX::get())
        }

        pub fn token_a_price_per_1e12_token_b_balance(&self) -> BalanceOf<T> {
            let token_a_priceper_token_b: u128 = self.token_a_priceper_token_b.saturated_into();
            let p = (token_a_priceper_token_b as f64 / T::OnePDEX::get() as f64) as u128;
            p.saturated_into()
        }
    }

    #[derive(Decode, Encode, Clone, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct InterestedInvestorInfo<T: Config + frame_system::Config> {
        account_id: T::AccountId,
        amount: BalanceOf<T>,
    }

    /// structure for storing voter information
    #[derive(Decode, Encode, Clone, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct Voter<T: Config> {
        pub account_id:  <T as frame_system::Config>::AccountId,
        pub votes: BalanceOf<T>,
    }

    /// structure for storing vote amount and unlocking block for a voter/investor
    #[derive(Decode, Encode, Clone, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct VoteCast<T: Config> {
        pub amount: BalanceOf<T>,
        pub unlocking_block: T::BlockNumber,
        pub voter_account: T::AccountId,
    }

    /// structure for storing vote for a round
    #[derive(Decode, Encode, Clone, TypeInfo)]
    #[scale_info(bounds(), skip_type_params(T))]
    pub struct Votes<T: Config> {
        pub ayes: Vec<Voter<T>>,
        pub nays: Vec<Voter<T>>,
    }

    impl<T: Config> Default for Votes<T> {
        fn default() -> Self {
            Votes { ayes: Vec::new(), nays: Vec::new() }
        }
    }

    #[pallet::pallet]
    #[pallet::without_storage_info]
    pub struct Pallet<T>(PhantomData<T>);

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(block_number: BlockNumberFor<T>) -> Weight {
            let call_weight: Weight = T::DbWeight::get().reads_writes(1, 1);
            <BallotReserve<T>>::mutate(|ballot_reserve| {
                let mut garbage = Vec::new();
                for (index, reserve) in ballot_reserve.iter().enumerate() {
                    if reserve.unlocking_block == block_number {
                        T::Currency::unreserve(&reserve.voter_account, reserve.amount);
                        garbage.push(index);
                        Self::deposit_event(Event::VoteAmountUnReserved(reserve.voter_account.clone(), reserve.amount));
                    }
                }
                for idx in garbage {
                    ballot_reserve.remove(idx);
                }
            });
            // Clean up WhiteListInvestors and InterestedParticipants in all expired rounds
            for (round_id, funding_round) in <InfoFundingRound<T>>::iter() {
                // TODO: This is commented out for now, Will be refactored and removed in the future 
                /* if block_number >= funding_round.vote_end_block {
                    let voting = <RoundVotes<T>>::get(&round_id);
                    let yes: BalanceOf<T> = voting.ayes.iter().map(|a| a.votes).fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                    let no: BalanceOf<T> = voting.nays.iter().map(|a| a.votes).fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                    if yes > no {
                        <WhitelistInfoFundingRound<T>>::insert(round_id.clone(), funding_round);
                        <InfoFundingRound<T>>::remove(&round_id);
                    } else {
                        <InfoFundingRound<T>>::remove(&round_id);
                        Self::deposit_event(Event::CleanedupExpiredRound(round_id));
                    }
                } */
            }


            // Loops through all approved funding rounds and checks if the funding round transfers funds from the investor to round creator
            for (round_id, funding_round) in <WhitelistInfoFundingRound<T>>::iter() {
                if block_number >= funding_round.close_round_block && !<InfoFundingRoundEnded<T>>::contains_key(round_id) {
                    let mut funding_round = funding_round.clone();
                    // Get all interested participants for a round
                    for (investor_address, amount) in <InterestedParticipants<T>>::iter_prefix(round_id) {
                        // Whitelist interested investor
                        <WhiteListInvestors<T>>::insert(round_id, investor_address.clone(), amount);
                        let total_raise = if T::OnePDEX::get().saturated_into::<BalanceOf<T>>() >= funding_round.token_a_priceper_token_b {
                            funding_round.token_a_price_per_1e12_token_b().mul_floor(funding_round.amount)
                        } else {
                            funding_round.token_a_price_per_1e12_token_b_balance().saturating_mul(funding_round.amount)
                        };

                        // Calculate investors share
                        let investor_share = Perquintill::from_rational_approximation(amount.saturated_into::<u64>(), total_raise.saturated_into::<u64>());
                        let round_account_id = Self::round_account_id(round_id.clone());

                        match Self::transfer(funding_round.token_b, &investor_address, &round_account_id, amount.saturated_into()) {
                            Ok(_) => {
                                <InvestorShareInfo<T>>::insert(round_id, investor_address.clone(), investor_share);
                                funding_round.actual_raise = funding_round.actual_raise.saturating_add(amount);
                                Self::deposit_event(Event::ParticipatedInRound(round_id, investor_address));
                            }
                            Err(error) => {
                                Self::deposit_event(Event::ParticipatedInRoundFailed(round_id, investor_address, error));
                            }
                        }
                    }
                    <WhitelistInfoFundingRound<T>>::insert(round_id.clone(), funding_round);
                    <InfoFundingRoundEnded<T>>::insert(round_id, true);
                }
            }
            return call_weight;
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Registers a new investor to allow participating in funding round.
        ///
        /// # Parameters
        ///
        /// * `origin`: Account to be registered as Investor
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn register_investor(origin: OriginFor<T>) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(!<InfoInvestor<T>>::contains_key(&who), Error::<T>::InvestorAlreadyRegistered);
            let amount: BalanceOf<T> = T::IDOPDXAmount::get();
            ensure!(T::Currency::reserve(&who, amount).is_ok(),Error::<T>::FailedToMoveBalanceToReserve);
            let unlocking_block = match <InvestorLockPeriod<T>>::try_get() {
                Ok(unlocking_period) => unlocking_period.saturating_add(current_block_no),
                Err(_) => T::DefaultVotingPeriod::get().saturating_add(current_block_no)
            };
            let data: InvestorLockData<T> = InvestorLockData::new(amount, unlocking_block);
            let investor_info = InvestorInfo {
                kyc_status: KYCStatus::Tier0,
                lock_data: Some(data),
            };

            <InfoInvestor<T>>::insert(who.clone(), investor_info.clone());
            Self::deposit_event(Event::InvestorRegistered(who.clone()));
            Self::deposit_event(Event::InvestorLockFunds(who, amount, unlocking_block));
            Ok(())
        }

        /// Unlocks investor locked fund for registering as investor
        /// # Parameters
        /// * origin : Investor Account

        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn investor_unlock_fund(origin: OriginFor<T>) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&who), Error::<T>::InvestorDoesNotExist);
            let mut investor_info: InvestorInfo<T> = <InfoInvestor<T>>::get(&who).ok_or(Error::<T>::InvestorDoesNotExist)?;
            ensure!(investor_info.lock_data.is_some(), <Error<T>>::AlreadyUnlockedInvestorRegistrationFunds);
            let lock_data = investor_info.lock_data.unwrap();
            ensure!(lock_data.unlock_block >= current_block_no, <Error<T>>::UnlockedInvestorRegistrationFundBlocked);
            T::Currency::unreserve(&who, lock_data.amount);
            investor_info.lock_data = None;
            <InfoInvestor<T>>::insert(who.clone(), investor_info.clone());
            Self::deposit_event(Event::InvestorUnLockFunds(who, lock_data.amount));
            Ok(())
        }

        /// Attests the investor to take part in the IDO pallet.
        /// Attestor is part of the governance committee of IDO pallet.
        ///
        /// # Parameters
        ///
        /// * `investor`: Registered investor
        /// * `kyc_status`: Level of KYC Status
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn attest_investor(origin: OriginFor<T>, investor: T::AccountId, kyc_status: KYCStatus) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor), <Error<T>>::InvestorDoesNotExist);
            InfoInvestor::<T>::mutate(&investor, |investor_info| {
                if let Some(ref mut investor_info) = investor_info {
                    investor_info.kyc_status = kyc_status;
                    Self::deposit_event(Event::InvestorAttested(investor.clone()));
                }
                Ok(())
            })
        }

        /// Registers a funding round with the amount as the total allocation for this round and vesting period.
        ///
        /// # Parameters
        /// * `cid` : IPFS cid
        /// * `token_a`: The Project token
        /// * `amount`: Amount for funding round
        /// * `token_b`: Token in which funding is received
        /// * `vesting_per_block`: Vesting per block
        /// * `funding_period`: Number of blocks from the current block for funding/show interest in funding round
        /// * `min_allocation`: Minimum allocation of funds investor can invest
        /// * `max_allocation`: Maximum allocation of funds investor can invest
        /// * `token_a_priceper_token_b`: Priceper amount for project token
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn register_round(
            origin: OriginFor<T>,
            cid: Vec<u8>,
            token_a: Option<AssetId>,
            amount: BalanceOf<T>,
            token_b: AssetId,
            vesting_per_block: BalanceOf<T>,
            funding_period: T::BlockNumber,
            min_allocation: BalanceOf<T>,
            max_allocation: BalanceOf<T>,
            token_a_priceper_token_b: BalanceOf<T>,
        ) -> DispatchResult {
            let team: T::AccountId = ensure_signed(origin)?;

            let (token_a, mintnew) = if let Some(token_a) = token_a {
                //TODO check if funder have the token_a available and reserve them.
                (token_a, false)
            } else {
                (Self::create_random_token()?, true)
                //TODO  mint the new token and reserve them.
                //TODO Make sure we mint the random tokens in an acceptable range.
            };

            let current_block_no = <frame_system::Pallet<T>>::block_number();

            let vote_end_block = match <VotingPeriod<T>>::try_get() {
                Ok(voting_period) => voting_period.saturating_add(current_block_no),
                Err(_) => T::DefaultVotingPeriod::get().saturating_add(current_block_no)
            };
            ensure!(token_a.ne(&token_b), <Error<T>>::TokenAEqTokenB);

            let start_block = vote_end_block.clone().saturating_add(1_u128.saturated_into());
            let close_round_block = vote_end_block.saturating_add(funding_period);
            // Ensures that
            let token_a_priceper_token_b_perquintill = Perbill::from_rational(token_a_priceper_token_b, 1_000_000_000_000_u128.saturated_into());

            // CID len must be less than or equal to 100
            ensure!(cid.len() <= 100, <Error<T>>::CidReachedMaxSize);
            ensure!(!token_a_priceper_token_b_perquintill.is_zero(), <Error<T>>::PricePerTokenCantBeZero);
            ensure!(min_allocation <= max_allocation, <Error<T>>::MinAllocationMustBeEqualOrLessThanMaxAllocation);
            ensure!(start_block < close_round_block, <Error<T>>::StartBlockMustBeLessThanEndblock);
            ensure!(vote_end_block < start_block, <Error<T>>::StartBlockMustBeGreaterThanVotingPeriod);
            ensure!(vesting_per_block > Zero::zero(), <Error<T>>::VestingPerBlockMustGreaterThanZero);


            // Mint random token if user selects none: TODO: Remove in production, only for beta testes
            ///TODO check if an old or new token again here and only mint the new

            let vesting_period: u32 = (amount / vesting_per_block).saturated_into();
            let vesting_period: T::BlockNumber = vesting_period.saturated_into();
            let vesting_end_block: T::BlockNumber = vesting_period.saturating_add(close_round_block);

            let funding_round: FundingRound<T> = FundingRound::from(
                cid,
                token_a,
                team.clone(),
                amount,
                token_b,
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

            //Charge minimum 1 PDEX required to create an account for the round account id
            T::Currency::transfer(&team, &round_account_id, T::ExistentialDeposit::get(), ExistenceRequirement::KeepAlive)?;

            // Transfers tokens to be released to investors from team account to round account
            // This ensure that the creator has the tokens they are raising funds for

            if mintnew {
                match token_a.clone() {
                    AssetId::asset(token_a) => {
                        T::AssetManager::create(token_a.into(), team.clone(), true, 1)?;
                        T::AssetManager::mint_into(token_a.into(), &round_account_id, amount.saturated_into())?;
                    }
                    _ => {
                        return Err(<Error<T>>::MintNativeTokenForbidden.into());
                    }
                }
            } else {
                Self::transfer(token_a, &team, &round_account_id, amount.saturated_into())?;
                //ensure!(.is_ok(), <Error<T>>::TransferTokenAFromTeamAccountFailed);
            }
            <WhitelistInfoFundingRound<T>>::insert(round_id, funding_round);
            <InfoProjectTeam<T>>::insert(team, round_id);
            Self::deposit_event(Event::FundingRoundRegistered(round_id));
            Ok(())
        }

        /// Project team whitelists investor for the given round for the given amount.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `investor_address`: Investor
        /// * `amount`: The max amount that investor will be investing in tokenB
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn whitelist_investor(origin: OriginFor<T>, round_id: T::Hash, investor_address: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
            let team: T::AccountId = ensure_signed(origin)?;
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(team.eq(&funding_round.creator), <Error<T>>::NotACreater);
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(current_block_no < funding_round.close_round_block && current_block_no >= funding_round.start_block, <Error<T>>::NotAllowed);
            <WhiteListInvestors<T>>::insert(round_id, investor_address.clone(), amount);
            Self::deposit_event(Event::InvestorWhitelisted(round_id, investor_address));
            Ok(())
        }

        /// Investor claiming for a particular funding round.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn claim_tokens(origin: OriginFor<T>, round_id: T::Hash) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id.clone()), Error::<T>::FundingRoundDoesNotExist);
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            // Investor can only withdraw after the funding round is closed
            let round_account_id = Self::round_account_id(round_id.clone());
            let investor_share = <InvestorShareInfo<T>>::get(round_id, investor_address.clone());
            // ensure the claiming block number falls with in the vesting period
            let claim_block: T::BlockNumber = min(current_block_no, funding_round.vesting_end_block);
            let total_released_block: T::BlockNumber = claim_block - funding_round.close_round_block;
            // total_tokens_released_for_given_investor is the total available tokens for their investment
            // relative to the current block
            let total_tokens_released_for_given_investor: BalanceOf<T> = investor_share.mul_floor(Self::block_to_balance(total_released_block)
                .saturating_mul(funding_round.vesting_per_block).saturated_into::<u64>()).saturated_into();

            //Check if investor previously claimed the tokens
            let claimed_tokens = if <InfoClaimAmount<T>>::contains_key(&round_id, &investor_address) {
                <InfoClaimAmount<T>>::get(&round_id, &investor_address)
            } else {
                Zero::zero()
            };
            // claimable_tokens : is the total amount of token the investor can withdraw(claim)  in their account
            let claimable_tokens = total_tokens_released_for_given_investor.saturating_sub(claimed_tokens);
            Self::transfer(funding_round.token_a, &round_account_id, &investor_address, claimable_tokens.saturated_into())?;

            <InfoClaimAmount<T>>::insert(round_id, investor_address.clone(), total_tokens_released_for_given_investor);
            // TODO : remove
            <LastClaimBlockInfo<T>>::insert(round_id, investor_address.clone(), current_block_no);
            Self::deposit_event(Event::TokenClaimed(round_id, investor_address));

            Ok(())
        }

        /// Stores information about investors, showing interest in funding round.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn invest(origin: OriginFor<T>, round_id: T::Hash, amount: BalanceOf<T>) -> DispatchResult {
            let investor_address: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&investor_address), <Error<T>>::InvestorDoesNotExist);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let mut funding_round = <WhitelistInfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundNotApproved)?;

            //Check If investor can invest amount
            ensure!(Self::can_withdraw(funding_round.token_b,&investor_address, amount.saturated_into()).is_ok(), Error::<T>::BalanceInsufficientForInteresetedAmount);
            // Max and Min allocation must be in token A to avoid the investor for under investing or over investing


            ///TODO make sure we have unit test for both paths.
            let amount_in_token_a = if T::OnePDEX::get().saturated_into::<BalanceOf<T>>() >= funding_round.token_a_priceper_token_b {
                funding_round.token_a_price_per_1e12_token_b().saturating_reciprocal_mul(amount)
            } else {
                amount / funding_round.token_a_price_per_1e12_token_b_balance()
            };
            //Ensure investment amount doesn't exceed max_allocation
            ensure!(amount_in_token_a <= funding_round.max_allocation && amount_in_token_a >= funding_round.min_allocation, Error::<T>::NotAValidAmount);

            let current_block_no = <frame_system::Pallet<T>>::block_number();
            // ensure!(current_block_no >= funding_round.start_block && current_block_no < funding_round.close_round_block, <Error<T>>::NotAllowed);

            let total_raise = funding_round.actual_raise;
            let round_account_id = Self::round_account_id(round_id.clone());

            // What is investor share? 
            let investor_share = Perquintill::from_rational_approximation(amount.saturated_into::<u64>(), total_raise.saturated_into::<u64>());

            // Transfer amounts to round account_id
            match Self::transfer(funding_round.token_b, &investor_address, &round_account_id, amount.saturated_into()) {
                Ok(_) => {
                    <InvestorShareInfo<T>>::insert(round_id, investor_address.clone(), investor_share);
                    funding_round.actual_raise = funding_round.actual_raise.saturating_add(amount);
                    Self::deposit_event(Event::ParticipatedInRound(round_id, investor_address));
                    <WhitelistInfoFundingRound<T>>::insert(round_id, funding_round);
                }
                Err(error) => {
                    Self::deposit_event(Event::ParticipatedInRoundFailed(round_id, investor_address, error));
                }
            }
            Ok(())
        }

        /// Transfers the raised amount to another address,
        /// only the round creator can call this or the governance.
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `beneficiary`: Account Id of Beneficiary
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn withdraw_raise(origin: OriginFor<T>, round_id: T::Hash, beneficiary: T::AccountId) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundDoesNotExist)?;
            ensure!(creator.eq(&funding_round.creator), <Error<T>>::NotACreater);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            let round_account_id = Self::round_account_id(round_id.clone());
            ensure!(Self::transfer(funding_round.token_b, &round_account_id, &beneficiary, funding_round.actual_raise.saturated_into()).is_ok(), Error::<T>::FundRaisedRedrawn);
            Self::deposit_event(Event::WithdrawRaised(round_id, creator));
            Ok(())
        }

        /// Vote for funding round to be whitelisted or not
        ///
        /// # Parameters
        ///
        /// * `round_id`: Funding round id
        /// * `amount`: Account Id of Beneficiary
        /// * `approve`: `true` approve `false` disapprove
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn vote(origin: OriginFor<T>, round_id: T::Hash, amount: BalanceOf<T>, vote_multiplier: u8, approve: bool) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            ensure!(vote_multiplier <=  6,  Error::<T>::PeriodError);
            ensure!(!amount.is_zero(),  Error::<T>::VoteCannotBeZero);
            let who: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <InfoFundingRound<T>>::get(&round_id).ok_or(Error::<T>::FundingRoundDoesNotExist)?;
            // ensure!(current_block_no < funding_round.vote_end_block , Error::<T>::VotingEnded);
            let mut voting = <RoundVotes<T>>::get(&round_id);
            let position_yes = voting.ayes.iter().position(|a| a.account_id == who);
            let position_no = voting.nays.iter().position(|a| a.account_id == who);

            //Reserves the vote amount will be later returned to user at vote.unlocking_block
            ensure!(T::Currency::reserve(&who, amount).is_ok(),Error::<T>::FailedToMoveBalanceToReserve);
            let unlocking_block = Self::vote_multiplier_to_block_number(vote_multiplier);
            let voter = Voter {
                account_id: who.clone(),
                votes: max(amount, amount.saturating_mul(vote_multiplier.saturated_into())),
            };
            let vote_cast = VoteCast {
                amount: amount.clone(),
                unlocking_block,
                voter_account: who.clone(),
            };
            <BallotReserve<T>>::mutate(|reserve| {
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

        /// Sets voting period for funding rounds (Governance Only)
        /// # Parameters
        /// * `period` : Number of blocks
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn set_vote_period(origin: OriginFor<T>, period: T::BlockNumber) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            <VotingPeriod<T>>::put(period);
            Ok(())
        }

        /// Sets investor fund lock period (Governance Only)
        /// # Parameters
        /// * `period` : Number of blocks
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn set_investor_lock_fund_period(origin: OriginFor<T>, period: T::BlockNumber) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            <InvestorLockPeriod<T>>::put(period);
            Ok(())
        }

        /// Force ido approval by governance (Governance Only)
        /// # Parameters
        /// * `round_id` : Round ID
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn approve_ido_round(origin: OriginFor<T>, round_id: T::Hash) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(!<WhitelistInfoFundingRound<T>>::contains_key(&round_id), <Error<T>>::RoundAlreadyApproved);
            ensure!(<InfoFundingRound<T>>::contains_key(&round_id), <Error<T>>::FundingRoundDoesNotExist);
            let funding_round = <InfoFundingRound<T>>::get(&round_id).ok_or(Error::<T>::FundingRoundDoesNotExist)?;
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
        #[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn withdraw_token(origin: OriginFor<T>, round_id: T::Hash, beneficiary: T::AccountId) -> DispatchResult {
            let current_block_no = <frame_system::Pallet<T>>::block_number();
            let creator: T::AccountId = ensure_signed(origin)?;
            ensure!(<InfoInvestor<T>>::contains_key(&beneficiary), <Error<T>>::InvestorDoesNotExist);
            ensure!(!<InfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundNotApproved);
            ensure!(<WhitelistInfoFundingRound<T>>::contains_key(&round_id), Error::<T>::FundingRoundDoesNotExist);
            let funding_round = <WhitelistInfoFundingRound<T>>::get(round_id).ok_or(Error::<T>::FundingRoundDoesNotExist)?;
            ensure!(creator.eq(&funding_round.creator), <Error<T>>::NotACreater);
            // Check if there is any left to withdraw
            let total_tokens_bought_by_investors = if T::OnePDEX::get().saturated_into::<BalanceOf<T>>() >= funding_round.token_a_priceper_token_b {
                funding_round.token_a_price_per_1e12_token_b().saturating_reciprocal_mul(funding_round.amount)
            } else {
                funding_round.amount / funding_round.token_a_price_per_1e12_token_b_balance()   //TODO saturated div
            };
            let remaining_token = funding_round.amount.saturating_sub(total_tokens_bought_by_investors);
            ensure!(current_block_no >= funding_round.close_round_block, Error::<T>::WithdrawalBlocked);
            ensure!(remaining_token > Zero::zero(), Error::<T>::WithdrawalBlocked);
            let round_account_id = Self::round_account_id(round_id.clone());
            //Transfers to remaining token back to creator after round.
            Self::transfer(funding_round.token_a, &round_account_id, &beneficiary, remaining_token.saturated_into())?;
            Self::deposit_event(Event::WithdrawToken(round_id, creator));
            Ok(())
        }
    }

    /// Stores investor Info
    #[pallet::storage]
    #[pallet::getter(fn get_investorinfo)]
    pub(super) type InfoInvestor<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        InvestorInfo<T>,
        OptionQuery,
    >;


    /// Stores project team/ ido creator Info
    #[pallet::storage]
    #[pallet::getter(fn get_team)]
    pub(super) type InfoProjectTeam<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        T::Hash,
        OptionQuery,
    >;

    /// Stores funding round info
    #[pallet::storage]
    #[pallet::getter(fn get_funding_round)]
    pub(super) type InfoFundingRound<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        FundingRound<T>,
        OptionQuery,
    >;


    /// Stores approved round info
    #[pallet::storage]
    #[pallet::getter(fn get_whitelist_funding_round)]
    pub(super) type WhitelistInfoFundingRound<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        FundingRound<T>,
        OptionQuery,
    >;

    /// Stores approved investor
    #[pallet::storage]
    #[pallet::getter(fn get_whitelist_investors)]
    pub(super) type WhiteListInvestors<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;


    /// Stores approved Investors share info for a specific round
    #[pallet::storage]
    #[pallet::getter(fn get_investor_share_info)]
    pub(super) type InvestorShareInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        Perquintill,
        ValueQuery,
    >;

    /// Stores last claimed block for and ido
    #[pallet::storage]
    #[pallet::getter(fn get_last_claim_block_info)]
    pub(super) type LastClaimBlockInfo<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        T::BlockNumber,
        ValueQuery,
    >;

    /// Stores total claimed token by an investor for a specific ido round
    #[pallet::storage]
    #[pallet::getter(fn get_claim_amount)]
    pub(super) type InfoClaimAmount<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    /// Stores interested participants for an ido round
    #[pallet::storage]
    #[pallet::getter(fn get_interested_particpants)]
    pub(super) type InterestedParticipants<T: Config> = StorageDoubleMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Blake2_128Concat,
        T::AccountId,
        BalanceOf<T>,
        ValueQuery,
    >;

    /// Stores interested partipants/investor amount will to be invested in the ido round
    #[pallet::storage]
    #[pallet::getter(fn get_interested_particpants_amounts)]
    pub(super) type InterestedParticipantsAmounts<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        BTreeMap<BalanceOf<T>, BTreeSet<T::AccountId>>,
        ValueQuery,
    >;

    // TODO: Remove
    #[pallet::storage]
    #[pallet::getter(fn get_funding_round_ended)]
    pub(super) type InfoFundingRoundEnded<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        bool,
        ValueQuery,
    >;

    /// Stores nonce used to create unique ido round id
    #[pallet::storage]
    #[pallet::getter(fn nonce)]
    pub(super) type Nonce<T: Config> = StorageValue<_, u128, ValueQuery>;


    /// Stores ido round voting period
    #[pallet::storage]
    #[pallet::getter(fn get_voting_period)]
    pub(super) type VotingPeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;

    /// Stores votes for ido round
    #[pallet::storage]
    #[pallet::getter(fn get_round_votes)]
    pub(super) type RoundVotes<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::Hash,
        Votes<T>,
        ValueQuery,
    >;


    /// Stores block and vote where investor will be refunded the amount reserved for votings
    #[pallet::storage]
    #[pallet::getter(fn get_ballot_reserve)]
    pub(super) type BallotReserve<T: Config> = StorageValue<_, Vec<VoteCast<T>>, ValueQuery>;

    /// Store the block to which investor can reclaim locked fund for registering as investor
    #[pallet::storage]
    #[pallet::getter(fn get_investor_period)]
    pub(super) type InvestorLockPeriod<T: Config> = StorageValue<_, T::BlockNumber, ValueQuery>;


    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Investor has been registered
        InvestorRegistered(<T as frame_system::Config>::AccountId),
        InvestorLockFunds(<T as frame_system::Config>::AccountId, BalanceOf<T>, <T as frame_system::Config>::BlockNumber),
        InvestorUnLockFunds(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        /// Investor has been attested
        InvestorAttested(<T as frame_system::Config>::AccountId),
        /// Funding round has been registered
        FundingRoundRegistered(T::Hash),
        /// Investor has been whitelisted
        InvestorWhitelisted(T::Hash, <T as frame_system::Config>::AccountId),
        /// Participant has been added
        ParticipatedInRound(T::Hash, <T as frame_system::Config>::AccountId),
        /// Token has been claimed
        TokenClaimed(T::Hash, <T as frame_system::Config>::AccountId),
        /// Showed interest in funding round
        ShowedInterest(T::Hash, <T as frame_system::Config>::AccountId),
        /// Transferred raised amount
        WithdrawRaised(T::Hash, <T as frame_system::Config>::AccountId),
        /// Transferred remaining tokens
        WithdrawToken(T::Hash, <T as frame_system::Config>::AccountId),
        /// IDO round has been removed from the storage
        CleanedupExpiredRound(T::Hash),
        /// IDO round whitelisted
        RoundWhitelisted(T::Hash),
        /// Investor vote amount reserved
        VoteAmountUnReserved(<T as frame_system::Config>::AccountId, BalanceOf<T>),
        /// IDO round participation failed
        ParticipatedInRoundFailed(T::Hash, <T as frame_system::Config>::AccountId, sp_runtime::DispatchError),
    }


    #[pallet::error]
    pub enum Error<T> {
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
        /// Start `start_block` is greater than `end_block`  error
        StartBlockMustBeLessThanEndblock,
        /// Start `start_block` is less than `voting_period`  error
        StartBlockMustBeGreaterThanVotingPeriod,
        /// Round already approved  error
        RoundAlreadyApproved,
        /// Attempt to redraw already redrawn raise amount
        FundRaisedRedrawn,
        /// invalid period
        PeriodError,
        /// attempting to vote for a round already ended voting period error
        VotingEnded,
        /// Attempt to vote more than once error
        DuplicateVote,
        /// Failed to move investor funds to reserve balance error
        FailedToMoveBalanceToReserve,
        /// Funding round not approved error
        FundingRoundNotApproved,
        /// CID bytes exceeding expected size error
        CidReachedMaxSize,
        /// `vesting_per_block` invalid error
        VestingPerBlockMustGreaterThanZero,
        /// Attempt to min native token in ido round creation error
        MintNativeTokenForbidden,
        /// Attempt to vote with 0 amount error
        VoteCannotBeZero,
        /// Investor attempting unlock already unlocked funds error
        AlreadyUnlockedInvestorRegistrationFunds,
        /// Investor attempting unlock investor registration fund when lock period is no expired yet
        UnlockedInvestorRegistrationFundBlocked,
        /// Insufficient balance in an account
        InsufficientBalance,
    }
}


impl<T: Config> Pallet<T> {
    /// module wallet account
    pub fn get_wallet_account() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// converts block to balance
    /// # Parameters
    /// * `input` : Block
    fn block_to_balance(input: T::BlockNumber) -> BalanceOf<T> {
        BalanceOf::<T>::from(input.saturated_into::<u32>())
    }

    /// Creates an accound id from round id
    /// # Parameters
    /// * hash : Round id
    pub fn round_account_id(hash: T::Hash) -> T::AccountId {
        T::ModuleId::get().into_sub_account(hash)
    }

    /// Increments and return a nonce
    fn incr_nonce() -> u128 {
        let current_nonce: u128 = <Nonce<T>>::get();
        let (nonce, _) = current_nonce.overflowing_add(1);
        <Nonce<T>>::put(nonce);
        <Nonce<T>>::get()
    }

    /// module wallet account
    pub fn pallet_account_id() -> T::AccountId {
        T::ModuleId::get().into_account()
    }

    /// Returns rounds an investor has invested in
    /// >  Used in RPC call
    /// # Paramteres
    /// * `account` : Account id
    pub fn rounds_by_investor(
        account: T::AccountId,
    ) -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        <InvestorShareInfo<T>>::iter()
            .filter_map(|(round_id, investor, _)| {
                if investor != account {
                    None
                } else {
                    if let Some(round_info) = <WhitelistInfoFundingRound<T>>::get(&round_id) {
                        Some((round_id, round_info.to_primitive()))
                    } else {
                        None
                    }
                }
            })
            .collect()
    }

    /// Returns rounds created by an account
    /// >  Used in RPC call
    /// # Paramteres
    /// * `account` : Account id
    pub fn rounds_by_creator(
        account: T::AccountId,
    ) -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        let whitelisted_funding_round: Vec<_> = <WhitelistInfoFundingRound<T>>::iter()
            .filter_map(|(round_id, round_info)| {
                if round_info.creator != account {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let pending_funding_round: Vec<_> = <InfoFundingRound<T>>::iter()
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

    /// Returns rounds that are not closed
    /// >  Used in RPC call
    pub fn active_rounds() -> Vec<(T::Hash, FundingRoundWithPrimitives<T::AccountId>)> {
        let current_block_no = <frame_system::Pallet<T>>::block_number();
        let mut active_rounds: Vec<_> = <WhitelistInfoFundingRound<T>>::iter()
            .filter_map(|(round_id, round_info)| {
                if round_info.close_round_block < current_block_no {
                    None
                } else {
                    Some((round_id, round_info.to_primitive()))
                }
            })
            .collect();

        let pending_funding_round: Vec<_> = <InfoFundingRound<T>>::iter()
            .map(|(round_id, round_info)| (round_id, round_info.to_primitive()))
            .collect();
        active_rounds.extend_from_slice(&pending_funding_round);
        active_rounds
    }
    /// Returns Votes statistics for a round
    /// >  Used in RPC call
    /// # Paramteres
    /// * `round_id` : Account id
    pub fn votes_stat(round_id: T::Hash) -> VoteStat {
        match <RoundVotes<T>>::try_get(&round_id) {
            Ok(voting) => {
                let yes: BalanceOf<T> = voting
                    .ayes
                    .iter()
                    .map(|a| a.votes)
                    .fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));
                let no: BalanceOf<T> = voting
                    .nays
                    .iter()
                    .map(|a| a.votes)
                    .fold(Zero::zero(), |sum, vote| sum.saturating_add(vote));

                VoteStat { yes: yes.saturated_into(), no: no.saturated_into() }
            }
            Err(_) => VoteStat { yes: 0, no: 0 },
        }
    }

    /// Calculates number of blocks for a vote amount will locked
    pub fn vote_multiplier_to_block_number(multiplier: u8) -> T::BlockNumber {
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

    /// Helper function to transfer tokens
    pub fn transfer(token: AssetId, from: &T::AccountId, to: &T::AccountId, amount: BalanceOf<T>) -> Result<(), sp_runtime::DispatchError> {
        match token {
            AssetId::polkadex => {
                T::Currency::transfer(from, to, amount, ExistenceRequirement::KeepAlive)
            }
            AssetId::asset(token_id) => {
                T::AssetManager::transfer(token_id, &from, &to, amount.saturated_into(), false).map(|_| ())
            }
        }
    }

    /// Helper function to check if investor can withdraw an amount
    pub fn can_withdraw(token: AssetId, from_account: &T::AccountId, amount: BalanceOf<T>) -> Result<(), sp_runtime::DispatchError> {
        match token {
            AssetId::polkadex => {
                let account_free_balance: u128 = T::Currency::free_balance(from_account)
                    .saturated_into();
                let new_balance = account_free_balance.checked_sub(amount.saturated_into())
                    .ok_or(Error::<T>::InsufficientBalance)?;
                T::Currency::ensure_can_withdraw(from_account, amount, WithdrawReasons::TRANSFER, new_balance.saturated_into())
            }
            AssetId::asset(token_id) => {
                T::AssetManager::can_withdraw(token_id.into(), from_account, amount.saturated_into()).into_result().map(|_| ())
            }
        }
    }

    /// Helper function to create a random token
    pub fn create_random_token() -> Result<AssetId, sp_runtime::DispatchError> {
        let seed = T::RandomnessSource::random_seed();
        let mut rng = ChaChaRng::from_seed(*seed.0.as_fixed_bytes());
        let random_asset_id: u128 = rng.gen();
        Ok(AssetId::asset(random_asset_id))
    }

    /// Takes a list of assets and Returns the asset balance(free balance) belonging to account_id
    pub fn account_balances(assets: Vec<u128>, account_id: T::AccountId) -> Vec<u128> {
        assets.iter().map(|asset| {
            <T as Config>::AssetManager::balance(*asset, &account_id).saturated_into()
        }).collect()
    }
}
