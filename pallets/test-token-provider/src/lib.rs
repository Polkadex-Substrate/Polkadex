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
		SignerNotFound,
		OffenderNotFound,
		BoundOverflow,
	}

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			let rng: u64 = 20;

			ValidTransaction::with_tag_prefix("thea-proc")
				.priority(rng)
				.and_provides([&(rng.to_be())])
				.longevity(3)
				.propagate(true)
				.build()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight((10_000, DispatchClass::Normal))]
		pub fn credit_account_with_tokens_unsigned(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Will this fail? 
			if let Ok(()) = T::AssetManager::mint_into(
				12,
				&account,
				100,
			){

			} else {
				T::AssetManager::create(
					12,
					account,
					true,
					100,
				)?;
			}
			// Code here to mint tokens
			Ok(().into())
		}
	}

	/* #[pallet::storage]
	#[pallet::getter(fn keygen_messages)]
	/// sender, KeygenRound => Messages
	pub(super) type KeygenMessages<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		PartyIndex,
		Blake2_128Concat,
		KeygenRound,
		TheaPayload<
			T::TheaId,
			KeygenRound,
			thea_primitives::MsgLimit,
			thea_primitives::MsgVecLimit,
		>,
		ValueQuery,
	*/

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {}


}
