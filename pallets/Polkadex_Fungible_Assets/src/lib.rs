#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_system::{self as system, ensure_signed};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult, Parameter,
};
use frame_support::sp_std::fmt::Debug;
use sp_runtime::traits::{AtLeast32BitUnsigned, StaticLookup, MaybeSerializeDeserialize, Member};
use sp_runtime::traits::Zero;

use sp_runtime::traits::CheckedSub;
use sp_runtime::traits::CheckedAdd;
use polkadex_primitives::assets::AssetId;
#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config: system::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Debug + MaybeSerializeDeserialize;
}

decl_event!(
	pub enum Event<T>
	where
		<T as system::Config>::AccountId,
		Balance = <T as Config>::Balance
	{
		TokenIssued(AssetId, AccountId, Balance),
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
			Ok(())
		}
	}
}
