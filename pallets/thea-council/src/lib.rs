// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

#![cfg_attr(not(feature = "std"), no_std)]

//! Thea Council Pallet
//!
//! Thea Council Pallet provides functionality to maintain council members on Parachain.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! Thea Council Pallet provides following functionalities:-
//!
//! - Adds member to Council.
//! - Removes member from Council.
//! - Block Transaction.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//! - `add_member` - Adds member to council.
//! - `remove_member` - Removes member from council.
//! - `claim_membership` - Converts Council member status from pending to Active.
//! - `delete_transaction` - Blocks withdrawal request.
//!
//! ### Public Inspection functions - Immutable (getters)
//! - `is_council_member` - Checks if given member is council member.
//!
//! ### Storage Items
//! - `ActiveCouncilMembers` - Stores Active Council Member List.
//! - `PendingCouncilMembers` - Stores Pending Council Member List.
//! - `Proposals` - Stores active proposals.
//! -
//! # Events
//! - `NewPendingMemberAdded` - New Pending Member added.
//! - `NewActiveMemberAdded` - New Active Member added.
//! - `MemberRemoved` - Council Member removed.
//! - `TransactionDeleted` - Transaction blocked.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::{Percent, SaturatedConversion};

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone)]
	pub enum Proposal<AccountId> {
		AddNewMember(AccountId),
		RemoveExistingMember(AccountId),
	}

	#[derive(Encode, Decode, TypeInfo, MaxEncodedLen, Copy, Clone, Eq, PartialEq, Debug)]
	pub struct Voted<AccountId>(pub AccountId);

	pub trait TheaCouncilWeightInfo {
		fn add_member(b: u32) -> Weight;
		fn remove_member(_b: u32) -> Weight;
		fn claim_membership(b: u32) -> Weight;
		fn delete_transaction(_b: u32) -> Weight;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + xcm_helper::Config {
		/// Because this pallet emits events, it depends on the Runtime's definition of an
		/// event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Minimum Active Council Size below witch Removal is not possible
		#[pallet::constant]
		type MinimumActiveCouncilSize: Get<u8>;
		/// How long pending council member have to claim membership
		#[pallet::constant]
		type RetainPeriod: Get<u64>;
		/// Wight Info
		type TheaCouncilWeightInfo: TheaCouncilWeightInfo;
	}

	/// Active Council Members
	#[pallet::storage]
	#[pallet::getter(fn get_council_members)]
	pub(super) type ActiveCouncilMembers<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, ConstU32<10>>, ValueQuery>;

	/// Pending Council Members
	#[pallet::storage]
	#[pallet::getter(fn get_pending_council_members)]
	pub(super) type PendingCouncilMembers<T: Config> =
		StorageValue<_, BoundedVec<(u64, T::AccountId), ConstU32<10>>, ValueQuery>;

	/// Proposals
	#[pallet::storage]
	#[pallet::getter(fn proposal_status)]
	pub(super) type Proposals<T: Config> = StorageMap<
		_,
		frame_support::Blake2_128Concat,
		Proposal<T::AccountId>,
		BoundedVec<Voted<T::AccountId>, ConstU32<10>>,
		ValueQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New Council Member Added [new_pending_member]
		NewPendingMemberAdded(T::AccountId),
		/// New active member added [new_active_member]
		NewActiveMemberAdded(T::AccountId),
		/// Member removed [member]
		MemberRemoved(T::AccountId),
		/// Transaction deleted
		TransactionDeleted(u32),
		/// Removed some unclaimed proposed council members
		RetainPeriodExpiredForCouncilProposal(u32),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Storage Overflow
		StorageOverflow,
		/// Not a Valid Sender
		BadOrigin,
		/// Already Council Member
		AlreadyMember,
		/// Not Pending Member
		NotPendingMember,
		/// Sender not council member
		SenderNotCouncilMember,
		/// Sender Already Voted
		SenderAlreadyVoted,
		/// Not Active Member
		NotActiveMember,
		/// Active Council Size is below Threshold
		ActiveCouncilSizeIsBelowThreshold,
		/// Proposals Storage Overflow
		ProposalsStorageOverflow,
		/// Pending Council Storage Overflow
		PendingCouncilStorageOverflow,
		/// Active Council Storage Overflow
		ActiveCouncilStorageOverflow,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Adds member to Thea Council.
		///
		/// # Parameters
		///
		/// * `new_member`: AccountId of New Member.
		#[pallet::call_index(0)]
		#[pallet::weight(T::TheaCouncilWeightInfo::add_member(1))]
		pub fn add_member(origin: OriginFor<T>, new_member: T::AccountId) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_council_member(&sender), Error::<T>::SenderNotCouncilMember);
			Self::do_add_member(sender, new_member)?;
			Ok(())
		}

		/// Removes member from Thea Council.
		///
		/// # Parameters
		///
		/// * `member_to_be_removed`: AccountId for memebr to be removed.
		#[pallet::call_index(1)]
		#[pallet::weight(T::TheaCouncilWeightInfo::remove_member(1))]
		pub fn remove_member(
			origin: OriginFor<T>,
			member_to_be_removed: T::AccountId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_council_member(&sender), Error::<T>::SenderNotCouncilMember);
			Self::do_remove_member(sender, member_to_be_removed)?;
			Ok(())
		}

		/// Converts Pending Council Member to Active Council Member.
		#[pallet::call_index(2)]
		#[pallet::weight(T::TheaCouncilWeightInfo::claim_membership(1))]
		pub fn claim_membership(origin: OriginFor<T>) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			Self::do_claim_membership(&sender)?;
			Self::deposit_event(Event::<T>::NewActiveMemberAdded(sender));
			Ok(())
		}

		/// Blocks malicious Pending Transaction.
		///
		/// # Parameters
		///
		/// * `block_no`: Block No which contains malicious transaction.
		/// * `index`: Index of Malicious transaction in the list.
		#[pallet::call_index(3)]
		#[pallet::weight(T::TheaCouncilWeightInfo::delete_transaction(1))]
		pub fn delete_transaction(
			origin: OriginFor<T>,
			block_no: BlockNumberFor<T>,
			index: u32,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(Self::is_council_member(&sender), Error::<T>::SenderNotCouncilMember);
			xcm_helper::Pallet::<T>::block_by_ele(block_no, index)?;
			Self::deposit_event(Event::<T>::TransactionDeleted(index));
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut removed = 0;
			<PendingCouncilMembers<T>>::mutate(|m| {
				let was = m.len();
				m.retain(|i| {
					T::RetainPeriod::get().saturating_add(i.0) >= n.saturated_into::<u64>()
				});
				removed = was.saturating_sub(m.len());
			});
			Self::deposit_event(Event::<T>::RetainPeriodExpiredForCouncilProposal(
				removed.saturated_into(),
			));
			T::DbWeight::get().reads_writes(1, removed.saturated_into())
		}
	}

	impl<T: Config> Pallet<T> {
		fn is_council_member(sender: &T::AccountId) -> bool {
			let active_members = <ActiveCouncilMembers<T>>::get();
			active_members.contains(sender)
		}

		fn is_pending_council_member(sender: &T::AccountId) -> bool {
			let pending_members = <PendingCouncilMembers<T>>::get();
			pending_members.iter().any(|m| m.1 == *sender)
		}

		fn do_add_member(sender: T::AccountId, new_member: T::AccountId) -> DispatchResult {
			ensure!(!Self::is_council_member(&new_member), Error::<T>::AlreadyMember);
			ensure!(!Self::is_pending_council_member(&new_member), Error::<T>::AlreadyMember);
			let proposal = Proposal::AddNewMember(new_member);
			Self::evaluate_proposal(proposal, sender)?;
			Ok(())
		}

		fn do_remove_member(
			sender: T::AccountId,
			member_to_be_removed: T::AccountId,
		) -> DispatchResult {
			let proposal = Proposal::RemoveExistingMember(member_to_be_removed);
			Self::evaluate_proposal(proposal, sender)?;
			Ok(())
		}

		pub(crate) fn get_expected_votes() -> usize {
			let total_active_council_size = <ActiveCouncilMembers<T>>::get().len();
			if total_active_council_size == 2 {
				2
			} else {
				let p = Percent::from_percent(65);
				p * total_active_council_size
			}
		}

		fn evaluate_proposal(
			proposal: Proposal<T::AccountId>,
			sender: T::AccountId,
		) -> DispatchResult {
			let current_votes =
				|votes: &BoundedVec<Voted<T::AccountId>, ConstU32<10>>| -> usize { votes.len() };
			let expected_votes = Self::get_expected_votes();
			let mut remove_proposal = false;
			<Proposals<T>>::try_mutate(proposal.clone(), |votes| {
				ensure!(!votes.contains(&Voted(sender.clone())), Error::<T>::SenderAlreadyVoted);
				votes
					.try_push(Voted(sender))
					.map_err(|_| Error::<T>::ProposalsStorageOverflow)?;
				if current_votes(votes) >= expected_votes {
					Self::execute_proposal(proposal.clone())?;
					remove_proposal = true;
				}
				Ok::<(), sp_runtime::DispatchError>(())
			})?;
			if remove_proposal {
				Self::remove_proposal(proposal);
			}
			Ok(())
		}

		fn remove_proposal(proposal: Proposal<T::AccountId>) {
			<Proposals<T>>::remove(proposal);
		}

		fn execute_proposal(proposal: Proposal<T::AccountId>) -> DispatchResult {
			match proposal {
				Proposal::AddNewMember(new_member) => Self::execute_add_member(new_member),
				Proposal::RemoveExistingMember(member_to_be_removed) =>
					Self::execute_remove_member(member_to_be_removed),
			}
		}

		fn execute_add_member(new_member: T::AccountId) -> DispatchResult {
			let mut pending_council_member = <PendingCouncilMembers<T>>::get();
			pending_council_member
				.try_push((
					<frame_system::Pallet<T>>::block_number().saturated_into(),
					new_member.clone(),
				))
				.map_err(|_| Error::<T>::PendingCouncilStorageOverflow)?;
			<PendingCouncilMembers<T>>::put(pending_council_member);
			Self::deposit_event(Event::<T>::NewPendingMemberAdded(new_member));
			Ok(())
		}

		fn execute_remove_member(member_to_be_removed: T::AccountId) -> DispatchResult {
			let mut active_council_member = <ActiveCouncilMembers<T>>::get();
			ensure!(
				active_council_member.len() > T::MinimumActiveCouncilSize::get().into(),
				Error::<T>::ActiveCouncilSizeIsBelowThreshold
			);
			let index = active_council_member
				.iter()
				.position(|member| *member == member_to_be_removed)
				.ok_or(Error::<T>::NotActiveMember)?;
			active_council_member.remove(index);
			<ActiveCouncilMembers<T>>::put(active_council_member);
			Self::deposit_event(Event::<T>::MemberRemoved(member_to_be_removed));
			Ok(())
		}

		fn do_claim_membership(sender: &T::AccountId) -> DispatchResult {
			let mut pending_members = <PendingCouncilMembers<T>>::get();
			let index = pending_members
				.iter()
				.position(|member| member.1 == *sender)
				.ok_or(Error::<T>::NotPendingMember)?;
			pending_members.remove(index);
			<PendingCouncilMembers<T>>::put(pending_members);
			let mut active_council_member = <ActiveCouncilMembers<T>>::get();
			active_council_member
				.try_push(sender.clone())
				.map_err(|_| Error::<T>::ActiveCouncilStorageOverflow)?;
			<ActiveCouncilMembers<T>>::put(active_council_member);
			Ok(())
		}
	}
}
