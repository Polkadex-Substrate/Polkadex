#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult,
    ensure,
};
use frame_system as system;
use frame_system::ensure_signed;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;
use codec::{Decode, Encode};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
}
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetMetadata {
    name: Vec<u8>,
    website: Vec<u8>,
    team: Vec<u8>
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetInfo<T: Config> {
    is_mintable: Option<T::AccountId>,
    is_burnable: Option<T::AccountId>,
    metadata: Option<AssetMetadata>,
    is_verified: bool
}

impl<T: Config> Default for AssetInfo<T> {
    fn default() -> Self {
        AssetInfo {
            is_mintable: None,
            is_burnable: None,
            metadata: None,
            is_verified: false
        }
    }
}

decl_storage! {
    trait Store for Module<T: Config> as PolkadexFungible {
        /// Stores AssetInfo
        InfoAsset get(fn get_infoasset): map hasher(identity) T::CurrencyId => AssetInfo<T>;
    }
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
		AssetIdAlreadyExists,
		AssetIdNotExists
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where
	origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

		/// Create new token.
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

        /// Info Fungible
        #[weight = 10000]
		pub fn set_metadata_fungible(origin, asset_id: T::CurrencyId, metadata: AssetMetadata) -> DispatchResult {
		    let who: T::AccountId = ensure_signed(origin)?;
		    // TODO: Ask @Gautam regarding who can add metadata
		    ensure!(<InfoAsset<T>>::contains_key(&asset_id), <Error<T>>::AssetIdNotExists);
		    InfoAsset::<T>::try_mutate(&asset_id, |ref mut asset_info| {
		        asset_info.metadata = Some(metadata);
		        Ok(())
		    })
		}

	}
}
