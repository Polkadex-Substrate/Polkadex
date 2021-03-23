#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, dispatch::DispatchResult,
    ensure,traits::{ExistenceRequirement, Get, Currency, EnsureOrigin}
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
    type TreasuryAccountId: Get<Self::AccountId>;
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;

}
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetMetadata {
    pub name: Vec<u8>,
    pub website: Vec<u8>,
    pub team: Vec<u8>
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetInfo<T: Config> {
    pub creator: T::AccountId,
    pub is_mintable: Option<T::AccountId>,
    pub is_burnable: Option<T::AccountId>,
    pub metadata: Option<AssetMetadata>,
    pub is_verified: bool
}

impl<T: Config> Default for AssetInfo<T> {
    fn default() -> Self {
        AssetInfo {
            creator: T::AccountId::default(),
            is_mintable: None,
            is_burnable: None,
            metadata: None,
            is_verified: false
        }
    }
}

impl<T: Config> AssetInfo<T> {
    fn from(creator: T::AccountId,is_mintable: Option<T::AccountId>, is_burnable: Option<T::AccountId>, metadata: Option<AssetMetadata>, is_verified: bool) -> Self {
        AssetInfo{
            creator,
            is_mintable,
            is_burnable,
            metadata,
            is_verified
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct VestingInfo<T: Config> {
    pub amount: T::Balance,
    pub rate: T::Balance,
    pub block_no: T::BlockNumber
}

impl<T: Config> Default for VestingInfo<T> {
    fn default() -> Self {
        VestingInfo{
            amount: T::Balance::default(),
            rate: T::Balance::default(),
            block_no: T::BlockNumber::default()
        }
    }
}
impl<T: Config> VestingInfo<T> {
    fn from(amount: T::Balance, rate: T::Balance, block_no: T::BlockNumber) -> Self {
        VestingInfo{
            amount,
            rate,
            block_no
        }
    }
}


decl_storage! {
    trait Store for Module<T: Config> as PolkadexFungible {
        /// Stores AssetInfo
        InfoAsset get(fn get_assetinfo): map hasher(identity) T::CurrencyId => AssetInfo<T>;
        InfoVesting get(fn get_vestinginfo): map hasher(identity) T::AccountId => VestingInfo<T>;
        FixedPDXAmount: T::Balance;
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
		AssetIdNotExists,
		VestingInfoExists,
		NotTheOwner,
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
						max_supply: T::Balance,
						mint_account: Option<T::AccountId>,
						burn_account: Option<T::AccountId>,
						existenial_deposit: T::Balance) -> DispatchResult {
						let who: T::AccountId = ensure_signed(origin)?;
						ensure!(!orml_tokens::TotalIssuance::<T>::contains_key(asset_id), Error::<T>::AssetIdAlreadyExists);
						ensure!(!<InfoAsset<T>>::contains_key(&asset_id), Error::<T>::AssetIdAlreadyExists);
						let tresury_account = T::TreasuryAccountId::get();
						let amout_to_trasfer: T::Balance = FixedPDXAmount::<T>::get();
//						orml_tokens::CurrencyAdapter::<T, Get<Currency<T::AccountId>>>::transfer(&who, &tresury_account, amout_to_trasfer, ExistenceRequirement::AllowDeath)?;
						let asset_info = AssetInfo::from(who.clone(), mint_account, burn_account, None, false);
						<InfoAsset<T>>::insert(asset_id, asset_info);
						orml_tokens::TotalIssuance::<T>::insert(asset_id, max_supply);
						let account_data = orml_tokens::AccountData{free: max_supply, reserved: T::Balance::zero(), frozen: T::Balance::zero()};
						orml_tokens::Accounts::<T>::insert(who.clone(), asset_id, account_data);
                        Self::deposit_event(RawEvent::TokenIssued(asset_id, who, max_supply));
			Ok(())
		}

		/// Vesting
		#[weight = 10000]
		pub fn set_vesting_info(origin, amount: T::Balance, rate: T::Balance, account: T::AccountId) -> DispatchResult {
		    /// Who can use this function?
		    /// From where balace is coming or is it minting
		    ensure!(!<InfoVesting<T>>::contains_key(&account), <Error<T>>::VestingInfoExists);
		    let current_block_no = <system::Module<T>>::block_number();
		    let vesting_info = VestingInfo::from(amount, rate, current_block_no);
		    <InfoVesting<T>>::insert(account, vesting_info);
		    Ok(())
		}

        /// Set Metadata
        #[weight = 10000]
		pub fn set_metadata_fungible(origin, asset_id: T::CurrencyId, metadata: AssetMetadata) -> DispatchResult {
		    let who: T::AccountId = ensure_signed(origin)?;
		    ensure!(<InfoAsset<T>>::contains_key(&asset_id), <Error<T>>::AssetIdNotExists);
		    let creator: AssetInfo<T> = Self::get_assetinfo(asset_id);
		    ensure!(who == creator.creator, <Error<T>>::NotTheOwner);
		    InfoAsset::<T>::try_mutate(&asset_id, |ref mut asset_info| {
		        asset_info.metadata = Some(metadata);
		        Ok(())
		    })
		}

		 /// Attest Token
        #[weight = 10000]
        pub fn attest_token(origin, asset_id: T::CurrencyId) -> DispatchResult {
		    let who = T::GovernanceOrigin::ensure_origin(origin)?;
		    ensure!(<InfoAsset<T>>::contains_key(&asset_id), <Error<T>>::AssetIdNotExists);
		    InfoAsset::<T>::try_mutate(&asset_id, |ref mut asset_info| {
		        asset_info.is_verified = true;
		        Ok(())
		    })
		}
	}
}
