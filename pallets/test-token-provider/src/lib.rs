#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

// #[cfg(test)]
// mod test;

pub use pallet::*;
// use sp_core::H160;

#[derive(PartialEq, PartialOrd, Ord, Eq)]
pub enum Assets {
	TestDot = 1,
	TestEth = 2,
	TestBTC = 3,
	TestDoge = 4,
	TestBNB = 5,
	Unknown,
}
impl Assets {
	fn from_u8(origin: u8) -> Self {
		match origin {
			1 => Assets::TestDot,
			2 => Assets::TestEth,
			3 => Assets::TestBTC,
			4 => Assets::TestDoge,
			5 => Assets::TestBNB,
			_ => Assets::Unknown,
		}
	}
}

#[frame_support::pallet]
pub mod pallet {
	use crate::Assets;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			tokens::fungibles::{Create, Inspect, Mutate},
			Currency, Get, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	pub use sp_core::H160;
	use sp_runtime::{
		traits::{AccountIdConversion, AtLeast32BitUnsigned, One, UniqueSaturatedInto},
		SaturatedConversion,
	};
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
		NotAllowed,
	}

	const BLOCK_THRESHOLD: u64 = (24 * 60 * 60) / 6;

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// Need to create Block treshold
			let current_block_no: T::BlockNumber = <frame_system::Pallet<T>>::block_number();
			let valid_tx = |account: &T::AccountId, asset_id: u128| {
				let last_block_number: T::BlockNumber;
				if let Some(block) = Self::fetch_block_number(account, asset_id) {
					last_block_number = block;
				} else {
					return TransactionValidity::Err(TransactionValidityError::Invalid(
						InvalidTransaction::ExhaustsResources,
					))
				}
				// let last_block_number: T::BlockNumber = Self::fetch_block_number(&account,
				// asset_id).unwrap();
				if (last_block_number == 0_u64.saturated_into()) ||
					(current_block_no - last_block_number >= BLOCK_THRESHOLD.saturated_into())
				{
					ValidTransaction::with_tag_prefix("token-faucet")
						.priority(100)
						.and_provides([account])
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
				if (last_block_number == 0_u64.saturated_into()) ||
					(current_block_no - last_block_number >= BLOCK_THRESHOLD.saturated_into())
				{
					ValidTransaction::with_tag_prefix("native-token")
						.priority(100)
						.and_provides([account])
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
				Call::credit_account_with_tokens_unsigned { account, asset_id } =>
					valid_tx(account, *asset_id as u128),
				Call::credit_account_with_native_tokens_unsigned { account } =>
					valid_native_tx(account),
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
			asset_id: u16,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			if !(1..=5).contains(&asset_id) {
				return Err(Error::<T>::NotAllowed.into())
			}
			let asset: Assets = Assets::from_u8(asset_id as u8);
			if asset == Assets::Unknown {
				return Err(Error::<T>::NotAllowed.into())
			}
			Self::transfer_assets(&account, asset_id as u128)?;
			Self::deposit_event(Event::AccountCredited(account));

			// Code here to mint tokens
			Ok(().into())
		}

		#[pallet::weight((10_000, DispatchClass::Normal))]
		pub fn credit_account_with_native_tokens_unsigned(
			origin: OriginFor<T>,
			account: T::AccountId,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			NativeTokenMap::<T>::insert(&account, <frame_system::Pallet<T>>::block_number());
			//Mint account with free tokens
			T::Currency::deposit_creating(&account, T::TokenAmount::get());
			Self::deposit_event(Event::AccountCredited(account));
			Ok(().into())
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn token_map)]
	pub(super) type TokenFaucetMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_btc)]
	pub(super) type TokenBTC<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_eth)]
	pub(super) type TokenEth<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_doge)]
	pub(super) type TokenDoge<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_dot)]
	pub(super) type TokenDot<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn token_bnb)]
	pub(super) type TokenBNB<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn native_token_map)]
	pub(super) type NativeTokenMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::BlockNumber, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		AccountCredited(T::AccountId),
	}

	impl<T: Config> Pallet<T> {
		// *** Utility methods ***

		/// Provides an AccountId for the pallet.
		/// This is used both as an origin check and deposit/withdrawal account.
		pub fn account_id() -> T::AccountId {
			MODULE_ID.into_account_truncating()
		}

		pub fn transfer_assets(account: &T::AccountId, asset_id: u128) -> DispatchResult {
			if let Err(_e) = T::AssetManager::mint_into(asset_id, account, 1000000000000000) {
				// Handling Unknown Asset by creating the Asset
				T::AssetManager::create(
					asset_id,
					Self::account_id(),
					true,
					BalanceOf::<T>::one().unique_saturated_into(),
				)?;
				// Minting Test Ether into the Account
				T::AssetManager::mint_into(asset_id, account, 1000000000000000)?;
			}
			match asset_id {
				1_u128 => {
					TokenDot::<T>::insert(account, <frame_system::Pallet<T>>::block_number());
				},
				2_u128 => {
					TokenEth::<T>::insert(account, <frame_system::Pallet<T>>::block_number());
				},
				3_u128 => {
					TokenBTC::<T>::insert(account, <frame_system::Pallet<T>>::block_number());
				},
				4_u128 => {
					TokenDoge::<T>::insert(account, <frame_system::Pallet<T>>::block_number());
				},
				5_u128 => {
					TokenBNB::<T>::insert(account, <frame_system::Pallet<T>>::block_number());
				},
				_ => {
					// Do nothing
				},
			};

			Ok(())
		}
		pub fn fetch_block_number(account: &T::AccountId, asset: u128) -> Option<T::BlockNumber> {
			match asset {
				1_u128 => Some(<TokenDot<T>>::get(account)),
				2_u128 => Some(<TokenEth<T>>::get(account)),
				3_u128 => Some(<TokenBTC<T>>::get(account)),
				4_u128 => Some(<TokenDoge<T>>::get(account)),
				5_u128 => Some(<TokenBNB<T>>::get(account)),
				_ => None,
			}
		}

		///  Provides Ethers Asset Id for Test Ether
		pub fn asset_id() -> u128 {
			100
		}

		pub fn asset_id_test_eth() -> u128 {
			101
		}

		pub fn asset_id_test_bnb() -> u128 {
			102
		}

		pub fn asset_id_test_doge() -> u128 {
			103
		}

		pub fn asset_id_test_dot() -> u128 {
			104
		}
	}
}
