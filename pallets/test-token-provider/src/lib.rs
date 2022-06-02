#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		pallet_prelude::*,
		traits::{fungible::Mutate, Currency, Get, LockableCurrency, WithdrawReasons},
	};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_runtime::{
		traits::{BlockNumberProvider, Saturating, Zero},
		SaturatedConversion,
	};

	#[pallet::config]
	pub trait Config: frame_system::Config {
		
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(PhantomData<T>);

}