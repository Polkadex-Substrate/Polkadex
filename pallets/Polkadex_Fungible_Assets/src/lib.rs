#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult,
    ensure,
    traits::{Currency, EnsureOrigin, ExistenceRequirement, Get},
};
use frame_system as system;
use frame_system::{Account, ensure_signed};
use sp_runtime::DispatchError;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;
use sp_runtime::traits::Hash;
use orml_traits::arithmetic::{CheckedAdd, CheckedSub};
use orml_traits::BasicCurrency;
use orml_traits::MultiCurrency;
use polkadex_primitives::assets::AssetId;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

pub trait Config: system::Config + orml_tokens::Config {
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type TreasuryAccountId: Get<Self::AccountId>;
    type GovernanceOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    // Native
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetMetadata {
    pub name: Vec<u8>,
    pub website: Vec<u8>,
    pub team: Vec<u8>,
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AssetInfo<T: Config> {
    pub creator: T::AccountId,
    pub is_mintable: Option<T::AccountId>,
    pub is_burnable: Option<T::AccountId>,
    pub metadata: Option<AssetMetadata>,
    pub is_verified: bool,
}

impl<T: Config> Default for AssetInfo<T> {
    fn default() -> Self {
        AssetInfo {
            creator: T::AccountId::default(),
            is_mintable: None,
            is_burnable: None,
            metadata: None,
            is_verified: false,
        }
    }
}

impl<T: Config> AssetInfo<T> {
    fn from(
        creator: T::AccountId,
        is_mintable: Option<T::AccountId>,
        is_burnable: Option<T::AccountId>,
        metadata: Option<AssetMetadata>,
        is_verified: bool,
    ) -> Self {
        AssetInfo {
            creator,
            is_mintable,
            is_burnable,
            metadata,
            is_verified,
        }
    }
}

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct VestingInfo<T: Config> {
    pub amount: T::Balance,
    pub rate: T::Balance,
    pub block_no: T::BlockNumber,
}

impl<T: Config> Default for VestingInfo<T> {
    fn default() -> Self {
        VestingInfo {
            amount: T::Balance::default(),
            rate: T::Balance::default(),
            block_no: T::BlockNumber::default()
        }
    }
}

impl<T: Config> VestingInfo<T> {
    fn from(amount: T::Balance, rate: T::Balance, block_no: T::BlockNumber) -> Self {
        VestingInfo {
            amount,
            rate,
            block_no
        }
    }
}

pub type VestingIndex = u32;

decl_storage! {
    trait Store for Module<T: Config> as PolkadexFungible {
        /// Stores AssetInfo
        InfoAsset get(fn get_assetinfo): map hasher(identity) T::CurrencyId => AssetInfo<T>;
        InfoVesting get(fn get_vestinginfo): double_map hasher(identity) (T::AccountId,T::CurrencyId), hasher(identity) T::Hash  => VestingInfo<T>;
        FixedPDXAmount get(fn get_amount): T::Balance;
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
        MetadataAdded(CurrencyId, AccountId),
        AmountMinted(CurrencyId, AccountId, Balance),
        TokenVerified(CurrencyId),
        AmountBurnt(CurrencyId, AccountId, Balance),
        TokenDepositModified(Balance),
    }
);

decl_error! {
    pub enum Error for Module<T: Config> {
        AssetIdAlreadyExists,
        AssetIdNotExists,
        VestingInfoExists,
        NoPermissionToMint,
        NoPermissionToBurn,
        Underflow,
        Overflow,
        NotTheOwner
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
		    orml_currencies::NativeCurrencyOf::<T>::transfer(&who, &tresury_account, amout_to_trasfer);
		    // https://github.com/paritytech/substrate/blob/master/frame/vesting/src/lib.rs#L322
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
        pub fn set_vesting_info(origin, amount: T::Balance, asset_id: T::CurrencyId, rate: T::Balance, account: T::AccountId) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            let asset_info: AssetInfo<T> = <InfoAsset<T>>::get(asset_id);
            ensure!(asset_info.creator == who, Error::<T>::AssetIdAlreadyExists);
            let current_block_no = <system::Module<T>>::block_number();
            let vesting_info = VestingInfo::from(amount, rate, current_block_no);
            // Random Pallet
            let identifier = (&current_block_no, &vesting_info).using_encoded(T::Hashing::hash);
            <InfoVesting<T>>::insert((account, asset_id), identifier, vesting_info);
            Ok(())
        }

        ///Claim
        #[weight = 10000]
        pub fn claim_vesting(origin, identifier: T::Hash, asset_id: T::CurrencyId) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            let vesting: VestingInfo<T> = <InfoVesting<T>>::get((who, asset_id), identifier);
            let current_block_no = <system::Module<T>>::block_number();
            // block_diff = currect_block - vesting.block_no
            // amount = block_diff * vesting.rate;
            // let amount_to_be_released = if (amount > vesting.amount) vesting.amount else amount
            // let vestting.amount = vestting.amount.saturation_sub(amount);
            // Insert Vesting Info
            // Update free balace of who (free_balabce + amoun_to_be_released)

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
                Self::deposit_event(RawEvent::MetadataAdded(asset_id, who));
                Ok(())
            })
        }

        /// Minting
        #[weight = 10000]
        pub fn mint_fungible(origin,to: T::AccountId, asset_id: T::CurrencyId, amount: T::Balance) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            Self::mint_token(&who, &to,asset_id, amount)?;
            Self::deposit_event(RawEvent::AmountMinted(asset_id, who, amount));
            Ok(())
        }

        /// Burn
        #[weight = 10000]
        pub fn burn_fungible(origin, asset_id: T::CurrencyId, amount: T::Balance) -> DispatchResult {
            let who: T::AccountId = ensure_signed(origin)?;
            Self::burn_token(&who,asset_id, amount)?;
            Self::deposit_event(RawEvent::AmountBurnt(asset_id, who, amount));
            Ok(())
        }

        /// Attest Token
        #[weight = 10000]
        pub fn attest_token(origin, asset_id: T::CurrencyId) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            ensure!(<InfoAsset<T>>::contains_key(&asset_id), <Error<T>>::AssetIdNotExists);
            InfoAsset::<T>::try_mutate(&asset_id, |ref mut asset_info| {
                asset_info.is_verified = true;
                Self::deposit_event(RawEvent::TokenVerified(asset_id));
                Ok(())
            })
        }

        /// Modify Token Registration
        #[weight = 10000]
        pub fn modify_token_deposit_amount(origin, pdx_amount: T::Balance) -> DispatchResult {
            T::GovernanceOrigin::ensure_origin(origin)?;
            <FixedPDXAmount<T>>::put::<T::Balance>(pdx_amount);
            Self::deposit_event(RawEvent::TokenDepositModified(pdx_amount));
            Ok(())
        }

    }
}

impl<T: Config> Module<T> {
    pub fn mint_token(
        who: &T::AccountId,
        to: &T::AccountId,
        asset_id: T::CurrencyId,
        amount: T::Balance,
    ) -> Result<(), Error<T>> {
        let asset_info: AssetInfo<T> = <InfoAsset<T>>::get(asset_id);
        match asset_info.is_mintable {
            Some(account) if account == *who => {
                orml_tokens::TotalIssuance::<T>::try_mutate(&asset_id, |max_supply| {
                    *max_supply = max_supply
                        .checked_add(&amount)
                        .ok_or(<Error<T>>::Overflow)?;
                    orml_tokens::Accounts::<T>::try_mutate(to, asset_id, |account| {
                        account.free = account
                            .free
                            .checked_add(&amount)
                            .ok_or(<Error<T>>::Overflow)?;
                        Ok(())
                    })
                })
            }
            Some(_) => Err(<Error<T>>::NoPermissionToMint),
            None => Err(<Error<T>>::AssetIdNotExists),
        }
    }

    pub fn burn_token(
        who: &T::AccountId,
        asset_id: T::CurrencyId,
        amount: T::Balance,
    ) -> Result<(), Error<T>> {
        let asset_info: AssetInfo<T> = <InfoAsset<T>>::get(asset_id);
        match asset_info.is_burnable {
            Some(account) if account == *who => {
                orml_tokens::TotalIssuance::<T>::try_mutate(&asset_id, |max_supply| {
                    *max_supply = max_supply
                        .checked_sub(&amount)
                        .ok_or(<Error<T>>::Underflow)?;
                    orml_tokens::Accounts::<T>::try_mutate(who, asset_id, |account| {
                        account.free = account
                            .free
                            .checked_sub(&amount)
                            .ok_or(<Error<T>>::Underflow)?;
                        Ok(())
                    })
                })
            }
            Some(_) => Err(<Error<T>>::NoPermissionToBurn),
            None => Err(<Error<T>>::AssetIdNotExists),
        }
    }

    pub fn transfer_native(
        from: &T::AccountId,
        to: &T::AccountId,
        asset_id: T::CurrencyId,
        amount: T::Balance,
    ) -> DispatchResult {
        orml_tokens::Accounts::<T>::try_mutate(from, asset_id, |account_from| {
            orml_tokens::Accounts::<T>::try_mutate(to, asset_id, |account_to| {
                account_from.free = account_from
                    .free
                    .checked_sub(&amount)
                    .ok_or(<Error<T>>::Underflow)?;
                account_to.free = account_to
                    .free
                    .checked_add(&amount)
                    .ok_or(<Error<T>>::Overflow)?;
                Ok(())
            })
        })
    }
}
