#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    decl_error, decl_event, decl_module, decl_storage,
    dispatch::DispatchResult, Parameter,
    traits::EnsureOrigin,
};
use frame_support::sp_std::fmt::Debug;
use frame_system::{self as system, ensure_signed};
use sp_core::U256;
use sp_runtime::SaturatedConversion;
use sp_runtime::traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Member, StaticLookup};
use sp_runtime::traits::CheckedAdd;
use sp_runtime::traits::CheckedSub;
use sp_runtime::traits::Zero;
use sp_std::prelude::*;

use chainbridge::{ChainId, DepositNonce, ResourceId};
use polkadex_primitives::assets::AssetId;
pub use weights::WeightInfo;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;
mod banchmarking;
pub mod weights;

pub trait Config: system::Config + chainbridge::Config {
    type BridgeOrigin: EnsureOrigin<Self::Origin, Success=Self::AccountId>;
    type Event: From<Event<Self>> + Into<<Self as system::Config>::Event>;
    type Balance: Parameter + Member + AtLeast32BitUnsigned + Default + Copy + Debug + MaybeSerializeDeserialize;
    type WeightInfo: WeightInfo;
}

decl_storage! {
	trait Store for Module<T: Config> as Assets {
		pub TotalIssuance get(fn total_issuance): map hasher(blake2_128_concat) AssetId => T::Balance;
		pub Balances get(fn balances): double_map hasher(blake2_128_concat) AssetId, hasher(blake2_128_concat) T::AccountId => T::Balance;
	}
	add_extra_genesis {
		config(balances): Vec<(AssetId, T::AccountId, T::Balance)>;
		build(|config: &GenesisConfig<T>| {
			for &(ref asset_id, ref who, amount) in config.balances.iter() {
				let total_issuance: T::Balance = TotalIssuance::<T>::get(asset_id);
				TotalIssuance::<T>::insert(asset_id, total_issuance + amount);
				Balances::<T>::insert(asset_id, who, amount);
			}
		});
	}
}

decl_event!(
	pub enum Event<T>
	where
		<T as system::Config>::AccountId,
		Balance = <T as Config>::Balance
	{
		Transferred(AssetId, AccountId, AccountId, Balance),
	}
);

decl_error! {
	pub enum Error for Module<T: Config> {
		TotalIssuanceOverflow,
		TotalIssuanceUnderflow,
		BalanceOverflow,
		InsufficientBalance
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {

		type Error = Error<T>;

		fn deposit_event() = default;

		/// Transfer some free balance to another account.
		/// TODO:  change name of function
		#[weight = 1000]
		pub fn transfer_new(origin,
						asset_id: AssetId,
						dest: <T::Lookup as StaticLookup>::Source,
						amount: T::Balance) -> DispatchResult {
			let from = ensure_signed(origin)?;
			let to = T::Lookup::lookup(dest)?;
			if amount.is_zero() || from == to {
			return Ok(())
		}
		<Balances<T>>::try_mutate(asset_id, from, |from_balance| -> DispatchResult {
			<Balances<T>>::try_mutate(asset_id, to, |to_balance| -> DispatchResult {
				*from_balance = from_balance.checked_sub(&amount).ok_or(Error::<T>::InsufficientBalance)?;
				*to_balance = to_balance.checked_add(&amount).ok_or(Error::<T>::BalanceOverflow)?;
				Ok(())
			})
		})
		}

		/// Withdraw
		#[weight = 1000]
		pub fn withdraw(origin, dest_id: ChainId, resource_id: ResourceId, to: Vec<u8>, #[compact] amount: T::Balance) -> DispatchResult {
		    let withdrawer = ensure_signed(origin)?;
		    // TODO: Verify withdrawer
		    let amount: T::Balance = 4444u128.saturated_into::<T::Balance>();
		    let amount_u256 = U256::from(amount.saturated_into::<u128>());
	        let asset_id: AssetId = AssetId::CHAINSAFE(resource_id);
		    <Balances<T>>::try_mutate(asset_id, withdrawer, |withdrawer_balance| -> DispatchResult {
		        *withdrawer_balance = withdrawer_balance.checked_sub(&amount).ok_or(Error::<T>::InsufficientBalance)?;
                chainbridge::Module::<T>::transfer_fungible(dest_id, resource_id, to, amount_u256)?;
                Ok(())
		    })
		}

		///Register Asset
		#[weight = 1000]
		pub fn register_asset(origin, resource_id: ResourceId, #[compact] balance: T::Balance) -> DispatchResult {
		    let withdrawer = ensure_signed(origin)?;
		    let assetid = AssetId::CHAINSAFE(resource_id);
		    <TotalIssuance<T>>::insert(assetid, balance);
		    <Balances<T>>::insert(assetid, withdrawer, balance);
		    Ok(())

		}

		/// Minting
		#[weight = 1000]
		pub fn transfer(origin, recipient: T::AccountId, resource_id: ResourceId, #[compact] amount: T::Balance) -> DispatchResult {
		    let source = T::BridgeOrigin::ensure_origin(origin)?;
	        let asset_id: AssetId = AssetId::CHAINSAFE(resource_id);
		    <Balances<T>>::try_mutate(asset_id, recipient, |mint_balance| -> DispatchResult {
		        *mint_balance = mint_balance.checked_add(&amount).ok_or(Error::<T>::BalanceOverflow)?;
                Ok(())
		    })
		}
	}
}