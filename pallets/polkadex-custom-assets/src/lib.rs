#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{decl_error, decl_event, decl_module, decl_storage, dispatch, ensure, Parameter};
use frame_support::sp_std::fmt::Debug;
use frame_support::traits::{BalanceStatus, Currency, ExistenceRequirement, Get, Imbalance, LockableCurrency, LockIdentifier, ReservableCurrency, SignedImbalance, TryDrop, WithdrawReason, WithdrawReasons};
use frame_system::{self as system, ensure_signed, ensure_root};
use sp_arithmetic::{FixedPointNumber, FixedU128, traits::CheckedDiv};
use sp_arithmetic::traits::{AtLeast32BitUnsigned, Saturating, UniqueSaturatedFrom, UniqueSaturatedInto};
use sp_runtime::{DispatchError, DispatchResult};
use sp_runtime::traits::{CheckedAdd, CheckedSub};
use sp_runtime::traits::{MaybeSerializeDeserialize, Member, Zero};
use sp_runtime::traits::Hash;
use sp_std::{mem, result};
use sp_std::convert::TryInto;
use sp_std::ops::{BitOr, Div};
use sp_std::vec::Vec;
use pallet_idenity::Judgement;

pub use self::imbalances::{NegativeImbalance, PositiveImbalance};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;
mod imbalances;


/// Configure the pallet by specifying the parameters and types on which it depends.
pub trait Trait: frame_system::Trait + pallet_idenity::Trait {
    /// Because this pallet emits events, it depends on the runtime's definition of an event.
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    /// Defines how balance is represented to users
    type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Debug + MaybeSerializeDeserialize + sp_runtime::FixedPointOperand + sp_runtime::traits::Saturating;

    /// Max number of Locks allowed on an account
    type MaxLocks: Get<u32>;

    /// The minimum amount required to keep an account open.
    type ExistentialDeposit: Get<Self::Balance>;
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reasons {
    /// Paying system transaction fees.
    Fee = 0,
    /// Any reason other than paying system transaction fees.
    Misc = 1,
    /// Any reason at all.
    All = 2,
}

impl From<WithdrawReasons> for Reasons {
    fn from(r: WithdrawReasons) -> Reasons {
        if r == WithdrawReasons::from(WithdrawReason::TransactionPayment) {
            Reasons::Fee
        } else if r.contains(WithdrawReason::TransactionPayment) {
            Reasons::All
        } else {
            Reasons::Misc
        }
    }
}

impl BitOr for Reasons {
    type Output = Reasons;
    fn bitor(self, other: Reasons) -> Reasons {
        if self == other { return self }
        Reasons::All
    }
}


#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
pub struct AccountData {
    pub free_balance: FixedU128,
    pub reserved_balance: FixedU128,
    pub misc_frozen: FixedU128,
    pub fee_frozen: FixedU128,

}

impl AccountData {
    fn frozen(&self, reasons: Reasons) -> FixedU128 {
        match reasons {
            Reasons::All => self.misc_frozen.max(self.fee_frozen),
            Reasons::Misc => self.misc_frozen,
            Reasons::Fee => self.fee_frozen,
        }
    }
}

impl Default for AccountData {
    fn default() -> Self {
        AccountData {
            free_balance: FixedU128::from(0),
            reserved_balance: FixedU128::from(0),
            misc_frozen: FixedU128::from(0),
            fee_frozen: FixedU128::from(0),

        }
    }
}


pub trait AssetIdProvider {
    type AssetId;
    fn asset_id() -> Self::AssetId;
}

pub struct PolkadexNativeAssetIdProvider<T>(sp_std::marker::PhantomData<T>);

impl<T: Trait> AssetIdProvider for PolkadexNativeAssetIdProvider<T> {
    type AssetId = T::Hash;
    fn asset_id() -> Self::AssetId {
        <Module<T>>::native_asset_id()
    }
}

pub type NativeAssetCurrency<T> = AssetCurrency<T, PolkadexNativeAssetIdProvider<T>>;

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Permissions {
    SystemLevel,
    FoundationLevel,
    UserLevel,
}

#[derive(Encode, Decode, Clone, Copy, PartialEq, Eq, Debug)]
pub struct AssetInfo<T> where T: Trait {
    total_issuance: FixedU128,
    issuer: T::AccountId,
    permissions: Permissions,
    existential_deposit: FixedU128,
}

impl<T> Default for AssetInfo<T> where T: Trait {
    fn default() -> Self {
        AssetInfo {
            total_issuance: FixedU128::from(0),
            issuer: Default::default(), // TODO: Add System Account Here.
            permissions: Permissions::SystemLevel,
            existential_deposit: FixedU128::from(0),
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq)]
pub struct AssetCurrency<T, U>(sp_std::marker::PhantomData<T>, sp_std::marker::PhantomData<U>);

// TODO 2 methods left
impl<T, U> Currency<<T as frame_system::Trait>::AccountId> for AssetCurrency<T, U>
    where
        T: Trait,
        U: AssetIdProvider<AssetId=T::Hash>,
{
    type Balance = T::Balance;
    type PositiveImbalance = PositiveImbalance<T, U>;
    type NegativeImbalance = NegativeImbalance<T, U>;

    fn total_balance(who: &<T as frame_system::Trait>::AccountId) -> Self::Balance {
        let account_data: AccountData = <Balance<T>>::get(U::asset_id(), who);
        // let total: FixedU128 = account_data.free_balance.saturating_add(
        //   account_data.reserved_balance);
        let total: FixedU128 = account_data.free_balance.saturating_add(account_data.reserved_balance);
        <Module<T>>::convert_fixedU128_to_balance(total)
    }

    fn can_slash(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> bool {
        let converted_value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        let account_data: AccountData = <Balance<T>>::get(U::asset_id(), who);
        account_data.free_balance >= converted_value
    }

    fn total_issuance() -> Self::Balance {
        let asset_info: AssetInfo<T> = <Assets<T>>::get(U::asset_id());
        <Module<T>>::convert_fixedU128_to_balance(asset_info.total_issuance)
    }

    fn minimum_balance() -> Self::Balance {
        T::ExistentialDeposit::get()
    }


    fn burn(amount: Self::Balance) -> Self::PositiveImbalance {
        if amount.is_zero() { return PositiveImbalance::zero(); }
        let amount: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(amount);
        let mut balance_to_return: FixedU128 = FixedU128::from(0u128);
        <Assets<T>>::mutate(U::asset_id(), |ref mut asset_info: &mut AssetInfo<T>| {
            if asset_info.total_issuance >= amount {
                balance_to_return = amount;
            } else{
                balance_to_return = asset_info.total_issuance;
            }
            asset_info.total_issuance = asset_info.total_issuance.saturating_sub(amount)
        });
        let balance_to_return= <Module<T>>::convert_fixedU128_to_balance(balance_to_return);
        PositiveImbalance::new(balance_to_return)
    }

    // TODO [Doubt] @Krishna
    fn issue(amount: Self::Balance) -> Self::NegativeImbalance {
        if amount.is_zero() { return NegativeImbalance::zero(); }
        let amount = <Module<T>>::convert_balance_to_fixedU128(amount);
        <Assets<T>>::mutate(U::asset_id(), |ref mut asset_info: &mut AssetInfo<T>| {
            asset_info.total_issuance = asset_info.total_issuance.saturating_add(amount)
        });
        let amount = <Module<T>>::convert_fixedU128_to_balance(amount);
        NegativeImbalance::new(amount)
    }

    fn free_balance(who: &<T as frame_system::Trait>::AccountId) -> Self::Balance {
        let account_data: AccountData = <Balance<T>>::get(U::asset_id(), who);
        <Module<T>>::convert_fixedU128_to_balance(account_data.free_balance)
    }

    fn ensure_can_withdraw(who: &<T as frame_system::Trait>::AccountId, amount: Self::Balance, reasons: WithdrawReasons, new_balance: Self::Balance) -> DispatchResult {
        // TODO: Check for freeze flag in identity pallet .
        // TODO: Check if given amount is okay under given KYC status.
        // TODO: Also check for locks in this pallet
        if amount.is_zero() { return Ok(()) }
        let new_balance = <Module<T>>::convert_balance_to_fixedU128(new_balance);
        let account_data: AccountData = <Balance<T>>::get(U::asset_id(), who);
        let min_balance = account_data.frozen(reasons.into());
        ensure!(new_balance >= min_balance, Error::<T>::LiquidityRestrictions);
        Ok(())
    }

    fn transfer(source: &<T as frame_system::Trait>::AccountId, dest: &<T as frame_system::Trait>::AccountId, value: Self::Balance, existence_requirement: ExistenceRequirement) -> DispatchResult {
        if value.is_zero() || source == dest { return Ok(()); }
        match <Module<T>>::transfer(source, dest, U::asset_id(), &value, existence_requirement) {
            Ok(()) => Ok(()),
            Err(e) => Err(DispatchError::from(e))
        }
    }
    // TODO [Feature] @Krishna Deduct amount from free balance and return
    fn slash(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        if value.is_zero() { return (NegativeImbalance::zero(), Zero::zero()); }
        let value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        let mut amount_left: FixedU128 = FixedU128::from(0);
        let mut amount_to_return = FixedU128::from(0);

        <Balance<T>>::mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {
            if value > account_data.free_balance {
                amount_left = value - account_data.free_balance;
            }
            account_data.free_balance = account_data.free_balance.saturating_sub(value);
            if amount_left > FixedU128::from(0) {
                if amount_left > account_data.reserved_balance {
                    amount_to_return = amount_left - account_data.reserved_balance;
                }
                account_data.reserved_balance = account_data.reserved_balance.saturating_sub(amount_left);
            }

        });
        let value = <Module<T>>::convert_fixedU128_to_balance(value);
        let amount_to_return = <Module<T>>::convert_fixedU128_to_balance(amount_to_return);
        (NegativeImbalance::new(value - amount_to_return), amount_to_return)
    }
    // TODO [Feature] Mint Balance to free balance of who
    // TODO [Research] If who is not there then will it bloat KV database?
    fn deposit_into_existing(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> Result<Self::PositiveImbalance, DispatchError> {
        ensure!(<Balance<T>>::contains_key(U::asset_id(), &who), Error::<T>::AccountNotFound);
        let value: FixedU128= <Module<T>>::convert_balance_to_fixedU128(value);
        <Balance<T>>::try_mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {
            account_data.free_balance = account_data.free_balance.saturating_add(value);
            let value = <Module<T>>::convert_fixedU128_to_balance(value);
            Ok((PositiveImbalance::new(value)))
        })
    }
    // TODO [Feature] @Krishna
    // TODO [Doubt] Misc and free_frozen value
// TODO [Doubt] @Krishna Ask Gautham about new account
    fn deposit_creating(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> Self::PositiveImbalance {
        if value.is_zero() { return Self::PositiveImbalance::zero(); }
        let value: FixedU128= <Module<T>>::convert_balance_to_fixedU128(value);
        if <Balance<T>>::contains_key(U::asset_id(), &who) {
            <Balance<T>>::mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {
                account_data.free_balance = account_data.free_balance.saturating_add(value);
            });
            let value = <Module<T>>::convert_fixedU128_to_balance(value);
            PositiveImbalance::new(value)
        } else {
            let new_account: AccountData = AccountData {
                free_balance: value,
                reserved_balance: FixedU128::from(0),
                misc_frozen: FixedU128::from(0),
                fee_frozen: FixedU128::from(0),
            };
            <Balance<T>>::insert(U::asset_id(), who, new_account);
            let value = <Module<T>>::convert_fixedU128_to_balance(value);
            PositiveImbalance::new(value)
        }
    }

    fn withdraw(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance, reasons: WithdrawReasons, liveness: ExistenceRequirement) -> Result<Self::NegativeImbalance, DispatchError> {
        unimplemented!()
    }
// TODO look again later
    fn make_free_balance_be(who: &<T as frame_system::Trait>::AccountId, balance: Self::Balance) -> SignedImbalance<Self::Balance, Self::PositiveImbalance> {
        let account: AccountData = <Balance<T>>::get(U::asset_id(), who);
        let balance: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(balance);
        let original = account.free_balance;
        let imbalance = if original <= balance {
            let temp = <Module<T>>::convert_fixedU128_to_balance(balance - original);
            SignedImbalance::Positive(PositiveImbalance::new(temp))
        } else {
            let temp = <Module<T>>::convert_fixedU128_to_balance(original - balance);
            SignedImbalance::Negative(NegativeImbalance::new(temp))
        };
        <Balance<T>>::mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {
            account_data.free_balance = balance;
        });

        imbalance
    }
}


impl<T, U> ReservableCurrency<<T as frame_system::Trait>::AccountId> for AssetCurrency<T, U>
    where
        T: Trait,
        U: AssetIdProvider<AssetId=T::Hash>,
{
    fn can_reserve(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> bool {
        let value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        let account_data: AccountData = <Balance<T>>::get(U::asset_id(), who);
        account_data.free_balance >= value
    }

    fn slash_reserved(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> (Self::NegativeImbalance, Self::Balance) {
        if value.is_zero() { return (NegativeImbalance::zero(), Zero::zero()); }
        let value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        let mut amount_to_return: FixedU128 = FixedU128::from(0);
        <Balance<T>>::mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {
            if value > account_data.reserved_balance {
                amount_to_return = value - account_data.reserved_balance;
            }
            account_data.reserved_balance = account_data.reserved_balance.saturating_sub(value);
        });
        let value = <Module<T>>::convert_fixedU128_to_balance(value);
        let amount_to_return = <Module<T>>::convert_fixedU128_to_balance(amount_to_return);
        (NegativeImbalance::new(value - amount_to_return), amount_to_return)
    }

    fn reserved_balance(who: &<T as frame_system::Trait>::AccountId) -> Self::Balance {
        let account_data: AccountData = <Balance<T>>::get(U::asset_id(), who);
        <Module<T>>::convert_fixedU128_to_balance(account_data.reserved_balance)
    }

    fn reserve(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> DispatchResult {
        if value.is_zero() { return Ok(()); }
        let value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        <Balance<T>>::try_mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {
            account_data.free_balance = account_data.free_balance.checked_sub(&value).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
            account_data.reserved_balance = account_data.reserved_balance.checked_add(&value).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
            Ok(())
        })
    }
// TODO [Not Implemented] If the remaining reserved balance is less than ExistentialDeposit, it will invoke on_reserved_too_low and could reap the account.
    fn unreserve(who: &<T as frame_system::Trait>::AccountId, value: Self::Balance) -> Self::Balance {
        if value.is_zero() { return value; }
        let value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        let mut amount_to_return: FixedU128 = FixedU128::from(0);

        <Balance<T>>::mutate(U::asset_id(), who, |ref mut account_data: &mut AccountData| {

            if value > account_data.reserved_balance {
                amount_to_return = value - account_data.reserved_balance;
                account_data.reserved_balance = account_data.reserved_balance.saturating_sub(value);
                account_data.free_balance = account_data.free_balance.saturating_add(account_data.reserved_balance);
            } else {
                account_data.reserved_balance = account_data.reserved_balance.saturating_sub(value);
                account_data.free_balance = account_data.free_balance.saturating_add(value);
            }
        });
        let amount_to_return = <Module<T>>::convert_fixedU128_to_balance(amount_to_return);
        amount_to_return
    }

    fn repatriate_reserved(slashed: &<T as frame_system::Trait>::AccountId, beneficiary: &<T as frame_system::Trait>::AccountId, value: Self::Balance, status: BalanceStatus) -> Result<Self::Balance, DispatchError> {
        ensure!(<Balance<T>>::contains_key(U::asset_id(), beneficiary), Error::<T>::AddUnderflowOrOverflow); //change
        let value: FixedU128 = <Module<T>>::convert_balance_to_fixedU128(value);
        let mut balance_to_return: FixedU128 = FixedU128::from(0);
        <Balance<T>>::try_mutate(U::asset_id(), slashed, |ref mut account_data: &mut AccountData| {
            if value > account_data.reserved_balance {
                balance_to_return = value - account_data.reserved_balance;
            }
            account_data.reserved_balance = account_data.reserved_balance.saturating_sub(value);
            <Balance<T>>::mutate(U::asset_id(), beneficiary, |ref mut inner_account_data: &mut AccountData| {
                if status == BalanceStatus::Free {
                    inner_account_data.free_balance = inner_account_data.free_balance.saturating_add(value.saturating_sub(balance_to_return));
                } else {
                    inner_account_data.reserved_balance = inner_account_data.reserved_balance.saturating_add(value.saturating_sub(balance_to_return));
                }
            });
            let balance_to_return = <Module<T>>::convert_fixedU128_to_balance(balance_to_return);
            Ok(balance_to_return)
        })
    }
}

/// A single lock on a balance. There can be many of these on an account and they "overlap", so the
/// same balance is frozen by multiple locks.
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct BalanceLock<Balance> {
    /// An identifier for this lock. Only one lock may be in existence for each identifier.
    pub id: LockIdentifier,
    /// The amount which the free balance may not drop below when this lock is in effect.
    pub amount: Balance,
    /// If true, then the lock remains in effect even for payment of transaction fees.
    pub reasons: Reasons,
}

impl<T, U> LockableCurrency<<T as frame_system::Trait>::AccountId> for AssetCurrency<T, U>
    where
        T: Trait,
        U: AssetIdProvider<AssetId=T::Hash>,
{
    type Moment = T::BlockNumber;
    type MaxLocks = T::MaxLocks;

    // Set a lock on the balance of `who`.
    // Is a no-op if lock amount is zero or `reasons` `is_none()`.
    fn set_lock(
        id: LockIdentifier,
        who: &T::AccountId,
        amount: T::Balance,
        reasons: WithdrawReasons,
    ) {
        if amount.is_zero() || reasons.is_none() { return; }

        let mut new_lock = Some(BalanceLock { id, amount, reasons: reasons.into() });
        let mut locks = Module::<T>::locks(who).into_iter()
            .filter_map(|l: BalanceLock<T::Balance>| if l.id == id { new_lock.take() } else { Some(l) })
            .collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
       Module::<T>::update_locks(who, &locks[..]);
    }

    // Extend a lock on the balance of `who`.
    // Is a no-op if lock amount is zero or `reasons` `is_none()`.
    fn extend_lock(
        id: LockIdentifier,
        who: &T::AccountId,
        amount: T::Balance,
        reasons: WithdrawReasons,
    ) {
        if amount.is_zero() || reasons.is_none() { return; }

        let mut new_lock = Some(BalanceLock { id, amount, reasons: reasons.into() });
        let mut locks = Module::<T>::locks(who).into_iter().filter_map(|l: BalanceLock<T::Balance>|
            if l.id == id {
                new_lock.take().map(|nl| {
                    BalanceLock {
                        id: l.id,
                        amount: l.amount.max(nl.amount),
                        reasons: l.reasons | nl.reasons,
                    }
                })
            } else {
                Some(l)
            }).collect::<Vec<_>>();
        if let Some(lock) = new_lock {
            locks.push(lock)
        }
      Module::<T>::update_locks(who, &locks[..]);
    }

    fn remove_lock(
        id: LockIdentifier,
        who: &T::AccountId,
    ) {
        let mut locks = Module::<T>::locks(who);
        locks.retain(|l: &BalanceLock<T::Balance>| l.id != id);
      Module::<T>::update_locks(who, &locks[..]);
    }
}


decl_storage! {
	trait Store for Module<T: Trait> as PolkadexCustomAssets {
        pub Balance get(fn get_free_balance): double_map hasher(blake2_128_concat) T::Hash, hasher(blake2_128_concat) T::AccountId => AccountData;


        pub Assets get(fn get_total_issuance) build(|config: &GenesisConfig<T>| {
          let asset_info = AssetInfo {
               total_issuance: config.initial_balance * FixedU128::from((config.endowed_accounts.len() as u128)),
               issuer: Default::default(),
               permissions: Permissions::SystemLevel,
               existential_deposit: FixedU128::from(0) // TODO Fix this - to T::ExistentialDeposit::get()
          };
          config.assets.iter().map(|id| (id.clone(), asset_info.clone())).collect::<Vec<_>>()
        }): map hasher(blake2_128_concat) T::Hash => AssetInfo<T>;


        Nonce: u64;

        NativeAssetId get (fn native_asset_id): T::Hash;

		pub Locks get(fn locks): map hasher(blake2_128_concat) T::AccountId => Vec<BalanceLock<T::Balance>>;
	}
	add_extra_genesis {
	    config(assets): Vec<T::Hash>;
	    config(initial_balance): FixedU128;
	    config(endowed_accounts): Vec<T::AccountId>;
	    config(native_asset): T::Hash;


	    build(|config: &GenesisConfig<T>| {
	    <NativeAssetId<T>>::put(config.native_asset);
	    let account_data = AccountData {
	        free_balance: config.initial_balance,
	        reserved_balance: FixedU128::from(0),
	        misc_frozen: FixedU128::from(0),
	        fee_frozen: FixedU128::from(0),
	    };
	        config.assets.iter().for_each(|asset_id| {
	            config.endowed_accounts.iter().for_each(|account_id| {
	                <Balance<T>>::insert(asset_id, account_id, account_data)
	            });
	        });
	    });
	}

}


decl_event!(
	pub enum Event<T> where AccountId = <T as frame_system::Trait>::AccountId,
	                        AssetID = <T as frame_system::Trait>::Hash,
	                        Balance = <T as Trait>::Balance
	                        {
		/// Asset Transferred [AssetId, From, To, Amount]
		Transferred(AssetID,AccountId,AccountId,Balance),
		/// New Asset Created [AssetId, Issuer, Total Issuance]
		AssetCreated(AssetID,AccountId,FixedU128),
	}
);

// Errors inform users that something went wrong.
decl_error! {
	pub enum Error for Module<T: Trait> {
		/// Not enough balance.
		InsufficientBalance,
		/// Balance overflowed during transfer.
		TransferOverFlow,
		/// Overflow occured while minting token.
		MintOverFlow,
		/// UnderFlow occured while burning.
		BurnUnderFlow,
		/// Conversion between Balance type and FixedU128 Failed
		FixedU128ConversionFailed,
		/// AssetId already in use
		AssetIdInUse,
		/// SubUnderflowOrOverflow
		SubUnderflowOrOverflow,
		/// AddUnderflowOrOverflow
		AddUnderflowOrOverflow,
		/// Account not found
		AccountNotFound,
		/// IssuerIdDoesNotMatch
		IssuerIdDoesNotMatch,
		/// Existential deposit of token issued is less than total issuance
		TotalIssuanceLessThanExistentialDeposit,
		/// ExistentialDeposit
		ExistentialDeposit,
		/// LiquidityRestrictions
		LiquidityRestrictions,
		/// KeepAlive
		KeepAlive,
		/// AccountFrozenOrOutdated
		AccountFrozenOrOutdated
	}
}

decl_module! {

	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
		// Errors must be initialized if they are used by the pallet.
		type Error = Error<T>;

		// Events must be initialized if they are used by the pallet.
		fn deposit_event() = default;

        /// Function to create and issue new asset in the Polkadex Ecosystem. The issued token will
        /// be credited into caller's account.
		#[weight = 10000]
	    pub fn create_token(origin, total_issuance: T::Balance, minimum_deposit: T::Balance) -> dispatch::DispatchResult{
	        let sender = ensure_signed(origin)?;
	        Self::issue_token(sender.clone(),total_issuance, sender,minimum_deposit)?;
            Ok(())
	    }

        /// Function to list available token in the Polkadex Orderbook Engine and Swap Engine.
        /// The inputTokenAmount and inputPolkadexAmount will be used to determine the Swap price
        /// and deposited into the swap liquidity pool.
	    #[weight = 10000]
	    pub fn list_token(origin, asset_id: T::Hash, input_token_amount: T::Balance, input_polkadex_amount: T::Balance) -> dispatch::DispatchResult{
            // TODO: Call register_trading_pair in Orderbook engine
            // TODO: Call register_swap_pair in Swap Engine along with input_token_amount, input_polkadex_amount as reserves
            // TODO: Return the trading pair id and swap pair id
            Ok(())
	    }

	    /// Function to transfer assets between accounts
	    #[weight = 10000]
	    pub fn transfer_token(origin, asset_id: T::Hash, amount: T::Balance, to: T::AccountId) -> dispatch::DispatchResult{
	        let sender = ensure_signed(origin)?;

            Self::transfer(&sender.clone(),&to.clone(),asset_id,&amount,ExistenceRequirement::KeepAlive)?;


            Self::deposit_event(RawEvent::Transferred(asset_id, sender, to, amount));

	        Ok(())
	    }

	}
}

impl<T: Trait> Module<T> {
    // TODO [Doubt] @Krishna
    /// This functions is used to create a new asset in the system. if issuer is one of the registrars
    /// then it's permission level is FoundationLevel, if issuer is Polkadex Bridgeing pallet's account
    /// then it is System level and finally if issuer is not registrar and not Bridging pallet's account
    /// then it is UserLevel.
    /// It will create a new asset id increases the beneficiary's balance for this asset by total_issuance
    /// and save the total_issuance in AssetInfo
    fn issue_token(issuer: T::AccountId, total_issuance: T::Balance, beneficiary: T::AccountId, existential_deposit: T::Balance) -> Result<(), Error<T>> {
        ensure!(total_issuance>existential_deposit, Error::<T>::TotalIssuanceLessThanExistentialDeposit);
        let total_issuance= Self::convert_balance_to_fixedU128(total_issuance);
        let existential_deposit = Self::convert_balance_to_fixedU128(existential_deposit);
        let permission_of_issuer = Self::get_permission(&issuer);
        let nonce = Nonce::get(); // TODO: A better way to introduce randomness
        let asset_id = (nonce, issuer.clone(), total_issuance.clone()).using_encoded(<T as frame_system::Trait>::Hashing::hash);
        ensure!(!<Assets<T>>::contains_key(asset_id), Error::<T>::AssetIdInUse);
        let asset_info = AssetInfo {
            total_issuance,
            issuer: issuer.clone(),
            permissions: permission_of_issuer,
            existential_deposit,
        };
        let account_data = AccountData {
            free_balance: total_issuance,
            reserved_balance: FixedU128::from(0),
            fee_frozen: FixedU128::from(0),
            misc_frozen: FixedU128::from(0),
        };
        <Balance<T>>::insert(&asset_id, &beneficiary, &account_data);
        <Assets<T>>::insert(&asset_id, &asset_info);
        Nonce::put(nonce + 1);
        Self::deposit_event(RawEvent::AssetCreated(asset_id,issuer,total_issuance));
        Ok(())
    }

    /// Increase the total_issuance and deposits the minted tokens to beneficiary account.
    /// Should check if the token exists and the issuer has the required permissions to do it.
    pub fn mint_token(issuer: T::AccountId, beneficiary: &T::AccountId, asset_id: T::Hash, amount: T::Balance) -> Result<(), Error<T>> {
        //ensure!(<Balance<T>>::contains_key(&asset_id, &beneficiary), Error::<T>::AccountNotFound); // TODO: [RES] @Gautham I am not sure do we need it
        let amount = Self::convert_balance_to_fixedU128(amount);
        match Self::get_permission(&issuer) {
            Permissions::FoundationLevel => {
                <Assets<T>>::try_mutate(&asset_id, |ref mut asset_info: &mut AssetInfo<T>| {
                    asset_info.total_issuance = asset_info.total_issuance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
                    <Balance<T>>::try_mutate(&asset_id, &beneficiary, |ref mut inner_account_data: &mut AccountData| {
                        inner_account_data.free_balance = inner_account_data.free_balance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
                        Ok(())
                    })
                })
            }
            Permissions::SystemLevel | Permissions::UserLevel => {
                <Assets<T>>::try_mutate(&asset_id, |ref mut asset_info: &mut AssetInfo<T>| {
                    ensure!(asset_info.issuer == issuer, Error::<T>::IssuerIdDoesNotMatch);
                    asset_info.total_issuance = asset_info.total_issuance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
                    <Balance<T>>::try_mutate(&asset_id, &beneficiary, |ref mut inner_account_data: &mut AccountData| {
                        inner_account_data.free_balance = inner_account_data.free_balance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
                        Ok(())
                    })
                })
            }

        }
    }

    /// Decrease the total_issuance and burns the minted tokens to beneficiary account.
    /// Should check if the token exists and the issuer has the required permissions to do it.
    // TODO [Doubt] @Krishna What if amount is not present?
    // TODO [Doubt] @Krishna Ask Gautham about deduction of left mount from reserved balance
    pub fn burn_token(issuer: T::AccountId, beneficiary: &T::AccountId, asset_id: T::Hash, amount: T::Balance) -> Result<(), Error<T>> {
        let amount: FixedU128 = Self::convert_balance_to_fixedU128(amount);
        let mut amount_deduction = FixedU128::from(0);
        match Self::get_permission(&issuer) {
            Permissions::FoundationLevel => {
                <Balance<T>>::try_mutate(&asset_id, &beneficiary, |ref mut account_data: &mut AccountData| {
                    if amount > account_data.free_balance {
                        amount_deduction = amount.checked_sub(&account_data.free_balance).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
                    } else {
                        amount_deduction = amount;
                    }
                    account_data.free_balance = account_data.free_balance.saturating_sub(amount);
                    <Assets<T>>::try_mutate(&asset_id, |ref mut asset_info: &mut AssetInfo<T>| {
                        asset_info.total_issuance = asset_info.total_issuance.checked_sub(&amount_deduction).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
                        Ok(())
                    })
                })

            }
            Permissions::SystemLevel | Permissions::UserLevel => {
                <Balance<T>>::try_mutate(&asset_id, &beneficiary, |ref mut account_data: &mut AccountData| {
                    if amount > account_data.free_balance {
                        amount_deduction = amount.checked_sub(&account_data.free_balance).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
                    } else {
                        amount_deduction = amount;
                    }
                    account_data.free_balance = account_data.free_balance.saturating_sub(amount);
                    <Assets<T>>::try_mutate(&asset_id, |ref mut asset_info: &mut AssetInfo<T>| {
                        ensure!(asset_info.issuer == issuer, Error::<T>::IssuerIdDoesNotMatch);
                        asset_info.total_issuance = asset_info.total_issuance.checked_sub(&amount_deduction).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
                        Ok(())
                    })
                })
            }

        }

    }
    /// It moves the amount from FreeBalance of account to ReservedBalance
    pub fn reserve(account: &T::AccountId, asset_id: T::Hash, amount: T::Balance) -> DispatchResult {
        let amount = Self::convert_balance_to_fixedU128(amount);
        ensure!(<Balance<T>>::contains_key(&asset_id, &account), Error::<T>::AccountNotFound);
        <Balance<T>>::try_mutate(asset_id, account, |ref mut account_data: &mut AccountData| {
            account_data.free_balance = account_data.free_balance.checked_sub(&amount).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
            account_data.reserved_balance = account_data.reserved_balance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
            Ok(())
        })
    }
    /// It moves the amount from ReservedBalance of account to FreeBalance
    pub fn unreserve(account: &T::AccountId, asset_id: T::Hash, amount: T::Balance) -> DispatchResult {
        let amount = Self::convert_balance_to_fixedU128(amount);
        ensure!(<Balance<T>>::contains_key(&asset_id, &account), Error::<T>::AccountNotFound);
        <Balance<T>>::try_mutate(asset_id, account, |ref mut account_data: &mut AccountData| {
            account_data.free_balance = account_data.free_balance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
            account_data.reserved_balance = account_data.reserved_balance.checked_sub(&amount).ok_or(Error::<T>::SubUnderflowOrOverflow)?;
            Ok(())
        })
    }
    /// Returns the free balance of that account
    pub fn free_balance(account: &T::AccountId, asset_id: T::Hash) -> T::Balance {
        let account_detail = <Balance<T>>::get(asset_id, account);
        Self::convert_fixedU128_to_balance(account_detail.free_balance)
    }
    /// Returns the reserved balance of that account
    pub fn reserved_balance(account: &T::AccountId, asset_id: T::Hash) -> T::Balance {
        let account_detail = <Balance<T>>::get(asset_id, account);
        Self::convert_fixedU128_to_balance(account_detail.reserved_balance)
    }

    /// Executes the transfer but it checks for KYC, Freeze Flag, minimum balance requirements and also if assetID is native currency
    /// then check for liquidity restrictions in Locks coming under the LockableCurrency trait.
    pub fn transfer(sender: &T::AccountId, receiver: &T::AccountId, asset_id: T::Hash, amount: &T::Balance, existence_requirement: ExistenceRequirement) -> DispatchResult {
        if amount.is_zero() || sender == receiver { return Ok(()) }
        let amount:FixedU128 = Self::convert_balance_to_fixedU128(*amount);
        ensure!(<Balance<T>>::contains_key(&asset_id, &sender), Error::<T>::AccountNotFound);
        <Balance<T>>::try_mutate(&asset_id, sender, |ref mut sender_account_data: &mut AccountData| {
            <Balance<T>>::try_mutate(&asset_id, receiver, |ref mut receiver_account_data: &mut AccountData| {

                sender_account_data.free_balance = sender_account_data.free_balance.checked_sub(&amount).ok_or(Error::<T>::InsufficientBalance)?;
                receiver_account_data.free_balance = receiver_account_data.free_balance.checked_add(&amount).ok_or(Error::<T>::AddUnderflowOrOverflow)?;
                let asset_info: AssetInfo<T> = <Assets<T>>::get(&asset_id);
                ensure!(receiver_account_data.free_balance.saturating_add(receiver_account_data.reserved_balance) >= asset_info.existential_deposit, Error::<T>::ExistentialDeposit);
                let amount = Self::convert_fixedU128_to_balance(amount);
                Self::can_withdraw(sender, &amount)?;
                let allow_death = existence_requirement == ExistenceRequirement::AllowDeath;
                let allow_death = allow_death && system::Module::<T>::allow_death(sender);
                ensure!(allow_death || sender_account_data.free_balance >= asset_info.existential_deposit, Error::<T>::KeepAlive);
                Ok(())

            })
        })

    }

    /// Check for KYC, Freeze flag, minimum balance
    //TODO [Res] and also we can send free balace so that we can reduce one read
    // Implement after all other pallets are integrated
    pub fn can_withdraw (sender: &T::AccountId, amount: &T::Balance) -> DispatchResult {
        match pallet_idenity::Module::<T>::check_account_status(sender) {

            Judgement::KnownGood | Judgement::PolkadexFoundationAccount | Judgement::Reasonable => Ok(()),
            //Judgement::Default & amount>some_amount => Ok(()) //TODO - @gautham
            Judgement::Default => Ok(()),
            Judgement::OutOfDate | Judgement::Freeze => Err(Error::<T>::AccountFrozenOrOutdated)?,
        }
    }

    /// TODO: @Krishna we will get into these functions related to Swap later.
    /// TODO: We should not be using these functions as they are some dirty hacks
    /// TODO: I did to make it work with Swap Engine.
    /// Used by Swap engine, there will be a imbalance in custom_assets_pallet but overall in Polkadex will have same total number of tokens
    pub fn decrease_balance_swap(trader: &T::AccountId, token0: T::Hash, token1: T::Hash,
                                 token0_amount: &FixedU128, token1_amount: &FixedU128) -> Result<(), Error<T>> {
        Ok(())
    }

    /// TODO: @Krishna we will get into these functions related to Swap later.
    /// TODO: We should not be using these functions as they are some dirty hacks
    /// TODO: I did to make it work with Swap Engine.
    /// Used by Swap engine, there will be a imbalance in custom_assets_pallet but overall Polkadex will have same total number of tokens
    pub fn increase_balance_swap(trader: &T::AccountId, token0: T::Hash, token1: T::Hash,
                                 token0_amount: &FixedU128, token1_amount: &FixedU128) -> Result<(), Error<T>> {
        Ok(())
    }

    /// TODO: @Krishna we will get into these functions related to Swap later.
    /// TODO: We should not be using these functions as they are some dirty hacks
    /// TODO: I did to make it work with Swap Engine.
    /// Used by Swap engine, there will be a imbalance in custom_assets_pallet but overall Polkadex will have same total number of tokens
    pub fn transfer_decrease0_increase1(trader: &T::AccountId, token0: T::Hash, token1: T::Hash,
                                        token0_amount: &FixedU128, token1_amount: &FixedU128) -> Result<(), Error<T>> {

        Ok(())
    }

    fn update_locks(who: &T::AccountId, locks: &[BalanceLock<T::Balance>]) {
        if locks.len() as u32 > T::MaxLocks::get() {
            frame_support::debug::warn!(
                "Warning: A user has more currency locks than expected. \
				A runtime configuration adjustment may be needed."
            );
        }
        let _result = <Balance<T>>::try_mutate(Self::native_asset_id(), who, |account_data| -> Result<(), Error<T>>{
            account_data.misc_frozen = FixedU128::from(0);
            account_data.fee_frozen = FixedU128::from(0);
            for l in locks.iter() {
                if l.reasons == Reasons::All || l.reasons == Reasons::Misc {
                    account_data.misc_frozen = account_data.misc_frozen.max(Self::convert_balance_to_fixedU128(l.amount));
                }
                if l.reasons == Reasons::All || l.reasons == Reasons::Fee {
                    account_data.fee_frozen = account_data.fee_frozen.max(Self::convert_balance_to_fixedU128(l.amount));
                }
            }
            Ok(())
        });
        // TODO: [RESEARCH] No idea what this remaining code does @Krishna
        // TODO: Do we really need it? Looks like some kind of Optimization trick
        // let existed = Locks::<T>::contains_key(who);
        // if locks.is_empty() {
        //     Locks::<T>::remove(who);
        //     if existed {
        //         // TODO: use Locks::<T>::hashed_key
        //         // https://github.com/paritytech/substrate/issues/4969
        //         frame_system::Module::<T>::dec_ref(who);
        //     }
        // } else {
        //     Locks::<T>::insert(who, locks);
        //     if !existed {
        //         frame_system::Module::<T>::inc_ref(who);
        //     }
        }



    #[allow(non_snake_case)]
    pub fn convert_balance_to_fixedU128(amount: T::Balance) -> FixedU128 {
        let y = TryInto::<u128>::try_into(amount).ok();
        FixedU128::from(y.unwrap()).checked_div(&FixedU128::from(1_000_000_000_000)).unwrap()
    }

    #[allow(non_snake_case)]
    pub fn convert_fixedU128_to_balance(x: FixedU128) -> T::Balance {
        let balance_in_fixed_u128 = x.checked_div(&FixedU128::from(1000000)).unwrap();
        let balance_in_u128 = balance_in_fixed_u128.into_inner();
        UniqueSaturatedFrom::<u128>::unique_saturated_from(balance_in_u128)
    }


    // TODO: if accountid belong to registrars then it is FoundationLevel, if accountid equals bridge pallet
    // TODO: then it is SystemLevel and anything else is UserLevel.
    fn get_permission(accountid: &T::AccountId) -> Permissions {
        // unimplemented!()
        Permissions::SystemLevel
    }
}

