#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub use pallet::*;
// use sp_core::H160;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		PalletId,
		pallet_prelude::*,
		traits::{
			tokens::fungibles::{Create, Inspect, Mutate},
			Currency, Get, LockableCurrency, WithdrawReasons, ReservableCurrency
		},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, BlockNumberProvider, Saturating, Zero, AccountIdConversion, Dispatchable, One, UniqueSaturatedInto},
		SaturatedConversion,
	};
	pub use sp_core::H160;
	// use core::str::FromStr;

	const MODULE_ID: PalletId = PalletId(*b"token/bg");

	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Responsible for minting tokens
		type AssetManager: Create<<Self as frame_system::Config>::AccountId>
			+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
			+ Inspect<<Self as frame_system::Config>::AccountId>;
		/// Balance Type
		type Balance: Parameter
			+ Member
			+ AtLeast32BitUnsigned
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Default token amount to mint
		type TokenAmount: Get<BalanceOf<Self>>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::error]
	pub enum Error<T> {
		AccountAlreadyCredited,
	}

	const BLOCK_THRESHOLD: u64 = (24 * 60 * 60) / 6;

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Need to create Block treshold
			let current_block_no: T::BlockNumber = <frame_system::Pallet<T>>::block_number();
			let valid_tx = |account: &T::AccountId| {
				let last_block_number: T::BlockNumber = <TokenFaucetMap<T>>::get(account);
				if (last_block_number == 0_u64.saturated_into()) || (current_block_no - last_block_number >= BLOCK_THRESHOLD.saturated_into())
				{
					ValidTransaction::with_tag_prefix("token-faucet")
							.priority(100)
							.and_provides([&b"request_token_faucet".to_vec()])
							.longevity(3)
							.propagate(true)
							.build()
				} else {
					TransactionValidity::Err(TransactionValidityError::Invalid(
						InvalidTransaction::ExhaustsResources,
					))
				}
			};
			let valid_native_tx = |account: &T::AccountId| {
				let last_block_number: T::BlockNumber = <NativeTokenMap<T>>::get(account);
				if (last_block_number == 0_u64.saturated_into()) || (current_block_no - last_block_number >= BLOCK_THRESHOLD.saturated_into())
				{
					ValidTransaction::with_tag_prefix("token-faucet")
							.priority(100)
							.and_provides([&b"request_token_faucet".to_vec()])
							.longevity(3)
							.propagate(true)
							.build()
				} else {
					TransactionValidity::Err(TransactionValidityError::Invalid(
						InvalidTransaction::ExhaustsResources,
					))
				}
			};
		
			match call {
				Call::credit_account_with_tokens_unsigned {account} => {
					valid_tx(&account)
				},
				Call::credit_account_with_native_tokens_unsigned {account} => {
					valid_native_tx(&account)
				},
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((10_000, DispatchClass::Normal))]
		pub fn credit_account_with_tokens_unsigned(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResultWithPostInfo {
			let _ = ensure_none(origin)?;
			if let Err(e) = T::AssetManager::mint_into(
				Self::asset_id(),
				&account,
				100,
			){
				// Handling Unknown Asset by creating the Asset
				T::AssetManager::create(
					Self::asset_id(),
					Self::account_id(),
					true,
					BalanceOf::<T>::one().unique_saturated_into(),
				)?; 
				// Minting Test Ether into the Account
				T::AssetManager::mint_into(
					Self::asset_id(),
					&account,
					100,
				)?;
			} 
			TokenFaucetMap::<T>::insert(&account,<frame_system::Pallet<T>>::block_number());
			Self::deposit_event(Event::AccountCredited(account));

			// Code here to mint tokens
			Ok(().into())
		}
		#[pallet::weight((10_000, DispatchClass::Normal))]
        pub fn credit_account_with_native_tokens_unsigned(origin: OriginFor<T>, account: T::AccountId) -> DispatchResultWithPostInfo {
            let _ = ensure_none(origin)?;
            NativeTokenMap::<T>::insert(&account,<frame_system::Pallet<T>>::block_number());
            //Mint account with free tokens
            T::Currency::deposit_creating(&account,T::TokenAmount::get());
            Self::deposit_event(Event::AccountCredited(account));
            Ok(().into())
        }
	}

	#[pallet::storage]
	#[pallet::getter(fn token_map)]
	pub(super) type TokenFaucetMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		T::BlockNumber,
		ValueQuery,
	>;	

	#[pallet::storage]
	#[pallet::getter(fn native_token_map)]
	pub(super) type NativeTokenMap<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		T::BlockNumber,
		ValueQuery,
	>;	

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		AccountCredited(T::AccountId)
	}

	impl<T: Config> Pallet<T> {
        // *** Utility methods ***

        /// Provides an AccountId for the pallet.
        /// This is used both as an origin check and deposit/withdrawal account.
        pub fn account_id() -> T::AccountId {
            MODULE_ID.into_account()
        }

		///  Provides Ethers Asset Id for Test Ether 
		pub fn asset_id() -> u128 {
			// Currently Hardcoding this value created from address "0xF59ae934f6fe444afC309586cC60a84a0F89Aaee"
			99237140875836081697465599727699073781
		}
	}

}
