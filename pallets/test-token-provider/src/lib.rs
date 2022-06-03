#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::fungibles::{Create, Inspect, Mutate},
			Currency, Get, LockableCurrency, WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{AtLeast32BitUnsigned, BlockNumberProvider, Saturating, Zero},
		SaturatedConversion,
	};
	// use frame_support::traits::tokens::nonfungibles::Create;

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
		
			match call {
				Call::credit_account_with_tokens_unsigned {account} => {
					valid_tx(&account)
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
			if let Ok(()) = T::AssetManager::mint_into(
				12,
				&account,
				1,
			){

			} else {
				T::AssetManager::create(
					12,
					account,
					true,
					1,
				)?;
			}
			// Code here to mint tokens
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

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {}



}
