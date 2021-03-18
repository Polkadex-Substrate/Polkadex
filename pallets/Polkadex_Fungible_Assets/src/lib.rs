#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_system as system;
use frame_support::{
    decl_error, decl_event, decl_module, ensure,
    dispatch::DispatchResult, Parameter,
};
use frame_support::sp_std::fmt::Debug;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member};

use polkadex_primitives::assets::AssetId;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
}

decl_event!(
	pub enum Event<T>
	where
		<T as system::Config>::AccountId,
	{
		TokenIssued(AssetId, AccountId),
	}
);

decl_error! {
	pub enum Error for Module<T: Config> {
		AssetIdAlreadyExists
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

		/// Transfer some free balance to another account.
		#[weight = 10000]
		pub fn create_token(origin,
						asset_id: AssetId,
						max_supply: T::Balance) -> DispatchResult {
						ensure!(!orml_tokens::TotalIssuance::<T>::contains_key(asset_id), Error::<T>::AssetIdAlreadyExists);

						orml_tokens::TotalIssuance::<T>::insert(asset_id, max_supply);
						orml_tokens::Accounts::<T>::insert(origin, asset_id, max_supply);
			Ok(())
		}
	}
}
