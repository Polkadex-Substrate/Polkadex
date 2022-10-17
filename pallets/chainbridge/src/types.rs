// Copyright 2021 ChainSafe Systems
// SPDX-License-Identifier: GPL-3.0-only

#![deny(warnings)]

use parity_scale_codec::u{
    Decode,
    Encode,
};
use frame_support::BoundedVec;
use frame_support::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_std::prelude::*;

pub type ChainId = u8;
pub type DepositNonce = u64;
pub type ResourceId = [u8; 32];

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ProposalStatus {
    Initiated,
    Approved,
    Rejected,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct ProposalVotes<AccountId, BlockNumber> {
    pub votes_for: BoundedVec<AccountId, VotesLimit>,
    pub votes_against: BoundedVec<AccountId, VotesLimit>,
    pub status: ProposalStatus,
    pub expiry: BlockNumber,
}

impl<AccountId, BlockNumber> Default for ProposalVotes<AccountId, BlockNumber>
where
    BlockNumber: Default,
{
    fn default() -> Self {
        Self {
            votes_for: BoundedVec::default(),
            votes_against: BoundedVec::default(),
            status: ProposalStatus::Initiated,
            expiry: BlockNumber::default(),
        }
    }
}

impl<AccountId, BlockNumber> ProposalVotes<AccountId, BlockNumber>
where
    AccountId: PartialEq,
    BlockNumber: PartialOrd,
{
    /// Attempts to mark the proposal as approve or rejected.
    /// Returns true if the status changes from active.
    pub(crate) fn try_to_complete(
        &mut self,
        threshold: u32,
        total: u32,
    ) -> ProposalStatus {
        if self.votes_for.len() >= threshold as usize {
            self.status = ProposalStatus::Approved;
            ProposalStatus::Approved
        } else if total >= threshold
            && (self.votes_against.len() as u32).saturating_add(threshold)
                > total
        {
            self.status = ProposalStatus::Rejected;
            ProposalStatus::Rejected
        } else {
            ProposalStatus::Initiated
        }
    }

    /// Returns true if the proposal has been rejected or approved, otherwise false.
    pub(crate) fn is_complete(&self) -> bool {
        self.status != ProposalStatus::Initiated
    }

    /// Returns true if the `who` has voted for or against the proposal
    pub(crate) fn has_voted(&self, who: &AccountId) -> bool {
        self.votes_for.contains(&who) || self.votes_against.contains(&who)
    }

    /// Returns true if the expiry time has been reached
    pub(crate) fn is_expired(&self, now: BlockNumber) -> bool {
        self.expiry <= now
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct VotesLimit;
impl Get<u32> for VotesLimit {
    fn get() -> u32 {
        20 // TODO: Arbitrary value
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub struct MethodLimit;
impl Get<u32> for MethodLimit {
    fn get() -> u32 {
        100 // TODO: Arbitrary value
    }
}


