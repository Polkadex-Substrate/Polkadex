#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;



#[frame_support::pallet]
pub mod pallet {
	use codec::{Decode, Encode, MaxEncodedLen};
	use frame_support::{
		pallet_prelude::*,
		traits::tokens::fungibles::{Create, Inspect, Mutate},
		PalletId,
	};
	use frame_support::traits::{ReservableCurrency, Currency, ExistenceRequirement};
	use frame_system::pallet_prelude::*;
	use scale_info::TypeInfo;
	use sp_core::H160;
	use sp_runtime::{
		traits::{BlockNumberProvider, Saturating, Zero},
		SaturatedConversion,
	};
	use sp_core::U256;
	use chainbridge::{ResourceId, BridgeChainId};
	use sp_runtime::traits::One;
	use sp_runtime::traits::UniqueSaturatedInto;
	type BalanceOf<T> =
	<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;



	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config: frame_system::Config + chainbridge::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
        /// Asset Manager
		type AssetManager: Create<<Self as frame_system::Config>::AccountId>
		+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
		+ Inspect<<Self as frame_system::Config>::AccountId>;

		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;

		/// Treasury PalletId
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// List of relayers who can relay data from Ethereum
	#[pallet::storage]
	#[pallet::getter(fn get_bridge_fee)]
	pub(super) type BridgeFee<T: Config> =
		StorageMap<_, Blake2_128Concat, BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;


	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Registered
		AssetRegistered(ResourceId),
		/// Asset Deposited (recipient, ResourceId, Amount)
		AssetDeposited(T::AccountId, ResourceId, BalanceOf<T>),
		/// Asset Withdrawn (recipient, ResourceId, Amount)
		AssetWithdrawn(H160, ResourceId, BalanceOf<T>),
		FeeUpdated(BridgeChainId, BalanceOf<T>, )
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Migration is not operational yet
		NotOperational,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(195_000_000)]
		pub fn create_asset(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			contract_add: H160
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let rid = chainbridge::derive_resource_id(chain_id, &contract_add.0);
			T::AssetManager::create(
				Self::convert_asset_id(rid),
				chainbridge::Pallet::<T>::account_id(),
				true,
				BalanceOf::<T>::one().unique_saturated_into(),
			)?;
			Self::deposit_event(Event::<T>::AssetRegistered(rid));
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		pub fn mint_asset(origin: OriginFor<T>, destination_add: T::AccountId, amount: BalanceOf<T>, rid: ResourceId) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			ensure!(sender == chainbridge::Pallet::<T>::account_id(), Error::<T>::NotOperational);
			T::AssetManager::mint_into(Self::convert_asset_id(rid), &destination_add, amount.saturated_into::<u128>())?;
			Self::deposit_event(Event::<T>::AssetDeposited(destination_add, rid, amount));
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		pub fn withdraw(origin: OriginFor<T>, chain_id: BridgeChainId, contract_add: H160, amount: BalanceOf<T>, recipient: H160) -> DispatchResult{
			let sender = ensure_signed(origin)?;
			let rid = chainbridge::derive_resource_id(chain_id, &contract_add.0);
			ensure!(T::AssetManager::reducible_balance(Self::convert_asset_id(rid), &sender, true)>=amount.saturated_into::<u128>(), Error::<T>::NotOperational);
			let fee = Self::fee_calculation(chain_id, amount);
			T::Currency::transfer(&sender, &sender, fee, ExistenceRequirement::KeepAlive)?;
			T::AssetManager::burn_from(Self::convert_asset_id(rid), &sender, amount.saturated_into::<u128>())?;
			chainbridge::Pallet::<T>::transfer_fungible(chain_id, rid, recipient.0.to_vec(), Self::convert_balance_to_eth_type(amount))?;
			Self::deposit_event(Event::<T>::AssetWithdrawn(contract_add, rid, amount));
			Ok(())
		}

		#[pallet::weight(195_000_000)]
		pub fn update_fee(origin: OriginFor<T>, chain_id: BridgeChainId, min_fee: BalanceOf<T>, fee_scale: u32) -> DispatchResult{
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<BridgeFee<T>>::insert(chain_id, (min_fee, fee_scale));
			Self::deposit_event(Event::<T>::FeeUpdated(chain_id, min_fee));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn remove_fradulent_tokens(beneficiary: T::AccountId) -> Result<(), DispatchError> {
			Ok(())
		}

		fn convert_balance_to_eth_type(balance: BalanceOf<T>) -> U256 {
			let balance: u128 = balance.unique_saturated_into();
			U256::from(balance).saturating_mul(U256::from(1000000u128))
		}

		fn fee_calculation(bridge_id: BridgeChainId, amount: BalanceOf<T>) -> BalanceOf<T>{
			let (min_fee, fee_scale) = Self::get_bridge_fee(bridge_id);
			let fee_estimated = amount * fee_scale.into() / 1000u32.into();
			if fee_estimated > min_fee {
				fee_estimated
			} else {
				min_fee
			}
		}

		pub fn convert_asset_id(token: ResourceId) -> u128 {
			let mut temp = [0u8; 16];
			temp.copy_from_slice(&token[0..16]);
			//temp.copy_fro	m_slice(token.as_fixed_bytes().as_ref());
			u128::from_le_bytes(temp)
		}
	}
}
