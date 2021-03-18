#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, dispatch::DispatchResult,
    ensure,
};
use frame_system as system;
use frame_system::ensure_signed;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

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
		<T as orml_tokens::Config>::CurrencyId,
		<T as orml_tokens::Config>::Balance
	{
		TokenIssued(CurrencyId, AccountId, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Config> {
		AssetIdAlreadyExists
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where
	origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

		/// Transfer some free balance to another account.
		#[weight = 10000]
		pub fn create_token(origin,
						asset_id: T::CurrencyId,
						max_supply: T::Balance) -> DispatchResult {
						let who: T::AccountId = ensure_signed(origin)?;
						ensure!(!orml_tokens::TotalIssuance::<T>::contains_key(asset_id), Error::<T>::AssetIdAlreadyExists);
						orml_tokens::TotalIssuance::<T>::insert(asset_id, max_supply);
						let account_data = orml_tokens::AccountData{free: max_supply, reserved: T::Balance::zero(), frozen: T::Balance::zero()};
						orml_tokens::Accounts::<T>::insert(who.clone(), asset_id, account_data);
                        Self::deposit_event(RawEvent::TokenIssued(asset_id, who, max_supply));
			Ok(())
		}
	}
}
