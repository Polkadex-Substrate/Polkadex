#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{pallet_prelude::*, traits::NamedReservableCurrency};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{BoundedBTreeMap, SaturatedConversion};
	type PublicKey = BoundedVec<u8, ConstU32<1000>>;
	type KeysMap = BoundedBTreeMap<u8, PublicKey, ConstU32<20>>;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_balances::Config + pallet_identity::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Stake required to apply for candidature
		type StakingAmount: Get<polkadex_primitives::Balance>;
		/// StakingReserveIdentifier
		#[pallet::constant]
		type StakingReserveIdentifier: Get<<Self as pallet_balances::Config>::ReserveIdentifier>;
		/// CouncilHandlerOrigin
		type CouncilHandlerOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::storage]
	#[pallet::getter(fn active_members)]
	/// Currently active thea council member
	pub(super) type ActiveMembers<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, KeysMap, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn candidates)]
	/// New candidates
	pub(super) type Candidates<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, KeysMap, OptionQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// New Candidate Added. [candidate]
		NewAccountAdded(T::AccountId),
		/// Candidate Approved. [candidate]
		CandidateApproved(sp_std::vec::Vec<T::AccountId>),
		/// New Keys Added [candidate]
		NewKeysAdded(T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Already Member
		AlreadyMember,
		/// Already applied
		AlreadyApplied,
		/// Candidate Not Found
		CandidateNotFound,
		/// Member Not Found
		MemberNotFound,
		/// Candidate doesnt have identity
		IdentityNotFound,
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Apply for candidature
		///
		/// # Parameters
		///
		///  `keys_list`: List of keys to be added.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(0)]
		pub fn apply_for_candidature(origin: OriginFor<T>, keys_list: KeysMap) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				pallet_identity::Pallet::<T>::has_identity(&who, 3),
				Error::<T>::IdentityNotFound
			);
			Self::do_apply(&who, keys_list)?;
			Self::deposit_event(Event::<T>::NewAccountAdded(who));
			Ok(())
		}

		/// Approve candidate request
		///
		/// # Parameters
		///
		/// * `new_keys`: List of candidates to be approved.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(1)]
		pub fn approve_candidature(
			origin: OriginFor<T>,
			candidate: sp_std::vec::Vec<T::AccountId>,
		) -> DispatchResult {
			T::CouncilHandlerOrigin::ensure_origin(origin)?;
			Self::do_approve(&candidate)?;
			Self::deposit_event(Event::<T>::CandidateApproved(candidate));
			Ok(())
		}

		/// Add keys for new Networks
		///
		/// # Parameters
		///
		/// * `new_keys`: Key Map to be removed.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(2)]
		pub fn add_new_keys(origin: OriginFor<T>, new_keys: KeysMap) -> DispatchResult {
			let member = ensure_signed(origin)?;
			Self::do_add_keys(&member, new_keys)?;
			Self::deposit_event(Event::<T>::NewKeysAdded(member));
			Ok(())
		}

		/// Remove from Active List
		///
		/// # Parameters
		///
		/// * `candidate`: List of Candidates to be removed.
		#[pallet::weight(Weight::default())]
		#[pallet::call_index(3)]
		pub fn remove_candidate(
			origin: OriginFor<T>,
			candidates: sp_std::vec::Vec<T::AccountId>,
		) -> DispatchResult {
			T::CouncilHandlerOrigin::ensure_origin(origin)?;
			Self::do_remove(&candidates)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn do_apply(who: &T::AccountId, keys_list: KeysMap) -> DispatchResult {
			ensure!(!<ActiveMembers<T>>::contains_key(who), Error::<T>::AlreadyMember);
			ensure!(!<Candidates<T>>::contains_key(who), Error::<T>::AlreadyApplied);
			let staking_amount = T::StakingAmount::get();
			pallet_balances::Pallet::<T>::reserve_named(
				&T::StakingReserveIdentifier::get(),
				who,
				staking_amount.saturated_into(),
			)?;
			<Candidates<T>>::insert(who, keys_list);
			Ok(())
		}

		#[frame_support::transactional]
		fn do_approve(candidates: &sp_std::vec::Vec<T::AccountId>) -> DispatchResult {
			for candidate in candidates {
				ensure!(!<ActiveMembers<T>>::contains_key(candidate), Error::<T>::AlreadyMember);
				if let Some(keys_map) = <Candidates<T>>::get(candidate) {
					<ActiveMembers<T>>::insert(candidate, keys_map);
				} else {
					return Err(Error::<T>::CandidateNotFound.into())
				}
			}
			Ok(())
		}

		fn do_add_keys(member: &T::AccountId, new_keys: KeysMap) -> DispatchResult {
			if let Some(existing_keys) = <ActiveMembers<T>>::get(member) {
				let mut inner_keys = existing_keys.into_inner();
				inner_keys.append(&mut new_keys.into_inner());
				let updated_keys: KeysMap = BoundedBTreeMap::try_from(inner_keys)
					.map_err(|_| Error::<T>::StorageOverflow)?;
				<ActiveMembers<T>>::insert(member, updated_keys);
				Ok(())
			} else {
				Err(Error::<T>::MemberNotFound.into())
			}
		}

		fn do_remove(candidates: &sp_std::vec::Vec<T::AccountId>) -> DispatchResult {
			for candidate in candidates {
				<Candidates<T>>::remove(candidate);
				<ActiveMembers<T>>::remove(candidate);
			}
			Ok(())
		}
	}
}
