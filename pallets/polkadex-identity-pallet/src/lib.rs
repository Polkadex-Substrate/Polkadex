#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    ensure,
    traits::Get,
};
use frame_system::{ensure_root, ensure_signed};
use sp_runtime::{DispatchResult, RuntimeDebug};
use sp_std::prelude::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod test;
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

pub type RegistrarIndex = u32;

pub struct FreezeAccount;

pub trait Config: frame_system::Config {
    /// The overarching event type.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;

    /// The maximum number of sub-accounts allowed per identified account.
    type MaxSubAccounts: Get<u32>;

    /// Maxmimum number of registrars allowed in the system. Needed to bound the complexity
    /// of, e.g., updating judgements.
    type MaxRegistrars: Get<u32>;

    /// Weight information for extrinsics in this pallet.
    type WeightInfo: WeightInfo;
}

#[derive(Copy, Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug)]
pub enum Judgement {
    /// The data appears to be reasonably acceptable in terms of its accuracy, however no in depth
    /// checks (such as in-person meetings or formal KYC) have been conducted.
    Reasonable,
    /// The target is known directly by the registrar and the registrar can fully attest to the
    /// the data's accuracy.
    KnownGood,
    /// The data was once good but is currently out of date. There is no malicious intent in the
    /// inaccuracy. This judgement can be removed through updating the data.
    OutOfDate,
    /// For Registrars
    PolkadexFoundationAccount,
    /// For default
    Default,
    /// Frozen Account
    Freeze,
}

impl Default for Judgement {
    fn default() -> Self {
        Self::Default
    }
}

decl_storage! {
	trait Store for Module<T: Config> as TemplateMo {

		pub IdentityOf get(fn identity):
			map hasher(blake2_128_concat) T::AccountId => Judgement;

		pub SuperOf get(fn super_of):
			map hasher(blake2_128_concat) T::AccountId => T::AccountId;

		pub SubsOf get(fn subs_of):
			map hasher(blake2_128_concat) T::AccountId => Vec<T::AccountId>;

		pub Registrars get(fn registrars):
			map hasher(blake2_128_concat) T::AccountId => Judgement;

	}
}
// TODO :- Remove unused variants
decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Config>::AccountId {
		/// A name was set or reset (which will remove all judgements). \[who\]
		IdentitySet(AccountId),
		/// A name was cleared, and the given balance returned. \[who, deposit\]
		IdentityCleared(AccountId),
		/// A name was removed and the given balance slashed. \[who, deposit\]
		IdentityKilled(AccountId),
		/// A judgement was asked from a registrar. \[who, registrar_index\]
		JudgementRequested(AccountId, RegistrarIndex),
		/// A judgement request was retracted. \[who, registrar_index\]
		JudgementUnrequested(AccountId, RegistrarIndex),
		/// A judgement was given by a registrar. \[target, registrar_index\]
		JudgementGiven(AccountId),
		/// A registrar was added. \[registrar_index\]
		RegistrarAdded(AccountId),
		/// A sub-identity was added to an identity and the deposit paid. \[sub, main, deposit\]
		SubIdentityAdded,
		/// A sub-identity was removed from an identity and the deposit freed.
		/// \[sub, main, deposit\]
		SubIdentityRemoved(AccountId, AccountId),
		/// A sub-identity was cleared, and the given deposit repatriated from the
		/// main identity account to the sub-identity account. \[sub, main, deposit\]
		SubIdentityRevoked(AccountId, AccountId),
		/// Trader added
		TraderAdded(AccountId),
		/// Account Frozen
		AccountFrozen,
		/// Sub Account Frozen
		SubAccountFrozen,
	}
);

decl_error! {
	/// Error for the identity module.
	pub enum Error for Module<T: Config> {
		/// Too many subs-accounts.
		TooManySubAccounts,
		/// Account isn't found.
		NotFound,
		/// Account isn't named.
		NotNamed,
		/// Empty index.
		EmptyIndex,
		/// Fee is changed.
		FeeChanged,
		/// No identity found.
		NoIdentity,
		/// Sticky judgement.
		StickyJudgement,
		/// Judgement given.
		JudgementGiven,
		/// Invalid judgement.
		InvalidJudgement,
		/// The index is invalid.
		InvalidIndex,
		/// The target is invalid.
		InvalidTarget,
		/// Too many additional fields.
		TooManyFields,
		/// Maximum amount of registrars reached. Cannot add any more.
		TooManyRegistrars,
		/// Account ID is already named.
		AlreadyClaimed,
		/// Sender is not a sub-account.
		NotSub,
		/// Sub-account isn't owned by sender.
		NotOwned,
		/// RegistrarAlreadyPresent.
		RegistrarAlreadyPresent,
		/// An identity is already present
		IdentityAlreadyPresent,
		/// GivenAccountIsSubAccount
		GivenAccountIsSubAccount,
		/// SenderIsNotRegistrar
		SenderIsNotRegistrar,
		/// GivenAccountNotRegistarar
		GivenAccountNotRegistarar,
		/// Frozen Account
		FrozenAccount,
		/// Not a free account
		NotFreeAccount,

	}
}

decl_module! {
    pub struct Module<T: Config> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;
        const MaxSubAccounts: u32 = T::MaxSubAccounts::get();
        const MaxRegistrars: u32 = T::MaxRegistrars::get(); //

    /// This method adds new Registrar
    /// # Arguments
    ///
    /// * `origin` - This contains the detail of Origin from where Transaction originated.
    ///
    /// * `account` - Account Id which is going to be new Registrar.
    ///
    /// # Return
    ///
    /// This function returns a status that, new Registrar is added or not.
    #[weight = 10000]
    fn add_registrar(origin, account: T::AccountId) -> DispatchResult {
        let _root_user = ensure_root(origin)?;
        ensure!(!<Registrars<T>>::contains_key(&account), Error::<T>::RegistrarAlreadyPresent); // Check for the existance
        let judgement = Judgement::PolkadexFoundationAccount;
        <Registrars<T>>::insert(&account, judgement);
        Self::deposit_event(RawEvent::RegistrarAdded(account));
        Ok(())
    }

    /// Using this method Registrar provides judgement to given account.
    /// # Arguments
    ///
    /// * `origin` - This contains the detail of Origin from where Transaction originated.
    ///
    /// * `target` - Account whose judgment is going to pass upon.
    ///
    /// * `judgement` - Judgement provided by Registrar for given account.
    ///
    /// # Return
    ///
    /// This function returns a status that, judgement is successfully passed or not.
    #[weight = 10000]
    fn provide_judgement_trader(origin,target: T::AccountId,judgement: Judgement) -> DispatchResult {
		    let registrar = ensure_signed(origin)?;
		    ensure!(<Registrars<T>>::contains_key(&registrar), Error::<T>::SenderIsNotRegistrar); // Check for the existance
		    ensure!(!SuperOf::<T>::contains_key(&target), Error::<T>::GivenAccountIsSubAccount);
			ensure!(!<IdentityOf<T>>::contains_key(&target), Error::<T>::IdentityAlreadyPresent); // Already present
			// Sub key already allocated
			<IdentityOf<T>>::insert(&target, judgement);
			Self::deposit_event(RawEvent::JudgementGiven(target));
			Ok(())
		}

	/// Using this method User can add associated account ids.
	/// # Arguments
	///
	/// * `origin` - This contains the detail of Origin from where Transaction originated.
	///
	/// * `sub_account` - Associated account which User wants to add.
	///
	/// # Return
	///
	/// This function returns a status that, sub account is successfully added or not.
	#[weight = 10000]
	fn add_sub_account(origin, sub_account: T::AccountId) -> DispatchResult {
	    let master_account = ensure_signed(origin)?;
	    ensure!(IdentityOf::<T>::contains_key(&master_account), Error::<T>::NoIdentity);
	    ensure!(!SuperOf::<T>::contains_key(&sub_account), Error::<T>::AlreadyClaimed);
        SubsOf::<T>::try_mutate(&master_account, |ref mut sub_ids| {
            ensure!(sub_ids.len() < T::MaxSubAccounts::get() as usize, Error::<T>::TooManySubAccounts);
            SuperOf::<T>::insert(&sub_account, master_account.clone());
            sub_ids.push(sub_account.clone());
            let judgement:Judgement = IdentityOf::<T>::get(&master_account);
            <IdentityOf<T>>::insert(&sub_account, judgement);
            Self::deposit_event(RawEvent::SubIdentityAdded);  // Change to sub account added
            Ok(())
        })
	}

    /// Using this method Registrar can freeze given account and all associated accounts as well.
    /// # Arguments
    ///
    /// * `origin` - This contains the detail of Origin from where Transaction originated.
    ///
    /// * `target` - Account which Registrar wants to freeze.
    ///
    /// # Return
    ///
    /// This function returns a status that, given account is successfully frozen or not.
	#[weight = 10000]
	fn freeze_account(origin, target: T::AccountId) -> DispatchResult {
			let registrar = ensure_signed(origin)?;
		    ensure!(<Registrars<T>>::contains_key(&registrar), Error::<T>::GivenAccountNotRegistarar); // Check for the existance
		    let account_to_freeze = if SuperOf::<T>::contains_key(&target) {
		        <SuperOf<T>>::get(&target)
		    } else {
		        target
		    };
		    if !IdentityOf::<T>::contains_key(&account_to_freeze) {
		        <IdentityOf<T>>::insert(&account_to_freeze, Judgement::Freeze);
		        Ok(())
		    }
		    else {
                let subs: Vec<T::AccountId> = <SubsOf<T>>::get(&account_to_freeze);
                <IdentityOf<T>>::mutate(&account_to_freeze, |ref mut judgement| {
                        **judgement = Judgement::Freeze;
                     });
                for sub_account in subs.iter() {
                     <IdentityOf<T>>::mutate(&sub_account, |ref mut judgement| {
                        **judgement = Judgement::Freeze;
                     });
                     Self::deposit_event(RawEvent::SubAccountFrozen);
                }
                Ok(())
               }
	    }
    }
}



// TODO :- Test this
impl<T: Config> Module<T> {
    pub fn check_account_status(account: &T::AccountId) -> Judgement {
        <IdentityOf<T>>::get(account)
    }

    pub fn is_registrar(account: T::AccountId) -> bool {
        <Registrars<T>>::contains_key(&account)
    }
}


