// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use crate::AssetHandlerWeightInfo;
	use chainbridge::{BridgeChainId, ResourceId};
	use frame_support::{
		dispatch::fmt::Debug,
		pallet_prelude::*,
		traits::{
			tokens::fungibles::{Create, Inspect, Mutate},
			Currency, ExistenceRequirement, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_core::{H160, U256};
	use sp_runtime::{
		traits::{One, Saturating, UniqueSaturatedInto, Zero},
		BoundedBTreeSet, SaturatedConversion,
	};
	use sp_std::vec::Vec;

	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
	pub struct PendingWithdrawal<
		Balance: Encode + Decode + MaxEncodedLen + Copy + Clone + Debug + Eq + PartialEq,
	> {
		pub chain_id: BridgeChainId,
		pub rid: ResourceId,
		pub amount: Balance,
		pub recipient: H160,
	}

	#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
	pub struct WithdrawalLimit;
	impl Get<u32> for WithdrawalLimit {
		fn get() -> u32 {
			5 // TODO: Arbitrary value
		}
	}
	#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode)]
	pub struct AllowlistedTokenLimit;

	impl Get<u32> for AllowlistedTokenLimit {
		fn get() -> u32 {
			50 // TODO: Arbitrary value
		}
	}

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

		type WeightInfo: AssetHandlerWeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	///Allowlisted tokens
	#[pallet::storage]
	#[pallet::getter(fn get_allowlisted_token)]
	pub(super) type AllowlistedToken<T: Config> =
		StorageValue<_, BoundedBTreeSet<H160, AllowlistedTokenLimit>, ValueQuery>;

	/// List of relayers who can relay data from Ethereum
	#[pallet::storage]
	#[pallet::getter(fn get_bridge_fee)]
	pub(super) type BridgeFee<T: Config> =
		StorageMap<_, Blake2_128Concat, BridgeChainId, (BalanceOf<T>, u32), ValueQuery>;

	///Block Difference required for Withdrawal Execution
	#[pallet::storage]
	#[pallet::getter(fn get_withdrawal_exc_block_diff)]
	pub(super) type WithdrawalExecutionBlockDiff<T: Config> =
		StorageValue<_, T::BlockNumber, ValueQuery>;

	///Block Difference required for Withdrawal Execution
	#[pallet::storage]
	#[pallet::getter(fn is_bridge_deactivated)]
	pub(super) type BridgeDeactivated<T: Config> = StorageValue<_, bool, ValueQuery>;

	/// Pending Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_pending_withdrawls)]
	pub(super) type PendingWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::BlockNumber,
		BoundedVec<PendingWithdrawal<BalanceOf<T>>, WithdrawalLimit>,
		ValueQuery,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Registered
		AssetRegistered(ResourceId),
		/// Asset Deposited (Recipient, ResourceId, Amount)
		AssetDeposited(T::AccountId, ResourceId, u128),
		/// Asset Withdrawn (Recipient, ResourceId, Amount)
		AssetWithdrawn(H160, ResourceId, BalanceOf<T>),
		FeeUpdated(BridgeChainId, BalanceOf<T>),
		/// NewBridgeStatus
		BridgeStatusUpdated(bool),
		/// BlocksDelayUpdated
		BlocksDelayUpdated(T::BlockNumber),
		/// FungibleTransferFailed
		FungibleTransferFailed,
		/// This token got allowlisted
		AllowlistedTokenAdded(H160),
		/// This token got removed from Allowlisted Tokens
		AllowlistedTokenRemoved(H160),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Migration is not operational yet
		NotOperational,
		/// MinterMustBeRelayer
		MinterMustBeRelayer,
		/// ChainIsNotAllowlisted
		ChainIsNotAllowlisted,
		/// NotEnoughBalance
		NotEnoughBalance,
		/// DestinationAddressNotValid
		DestinationAddressNotValid,
		/// DivisionUnderflow
		DivisionUnderflow,
		/// WithdrwalLimitReached
		WithdrawalLimitReached,
		/// ConversionIssue
		ConversionIssue,
		/// BridgeDeactivated
		BridgeDeactivated,
		/// Allowlisted token limit reached
		AllowlistedTokenLimitReached,
		/// This token is not Allowlisted
		TokenNotAllowlisted,
		/// This token was allowlisted but got removed and is not valid anymore
		AllowlistedTokenRemoved,
		/// Division Overflow
		DivisionOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// On Initialize
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let withdrawal_execution_block =
				n.saturating_sub(<WithdrawalExecutionBlockDiff<T>>::get());
			if !withdrawal_execution_block.is_zero() {
				let mut pending_withdrawals =
					<PendingWithdrawals<T>>::take(withdrawal_execution_block);
				for withdrawal in 0..pending_withdrawals.len() {
					if chainbridge::Pallet::<T>::transfer_fungible(
						pending_withdrawals[withdrawal].chain_id,
						pending_withdrawals[withdrawal].rid,
						pending_withdrawals[withdrawal].recipient.0.to_vec(),
						Self::convert_balance_to_eth_type(pending_withdrawals[withdrawal].amount),
					)
					.is_ok()
					{
						// Remove succesfull transfers
						pending_withdrawals.remove(withdrawal);
					} else {
						Self::deposit_event(Event::<T>::FungibleTransferFailed);
					}
				}
				// Write back to storage item
				<PendingWithdrawals<T>>::insert(withdrawal_execution_block, pending_withdrawals);
			}
			// TODO: Benchmark on initialize
			(195_000_000 as Weight)
				.saturating_add(T::DbWeight::get().writes(5 as Weight))
				.saturating_add(T::DbWeight::get().reads(5 as Weight))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Creates new Asset where AssetId is derived from chain_id and contract Address
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `chain_id`: Asset's native chain
		/// * `contract_add`: Asset's actual address at native chain
		#[pallet::weight(T::WeightInfo::create_asset(1))]
		pub fn create_asset(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			contract_add: H160,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let rid = chainbridge::derive_resource_id(chain_id, &contract_add.0);
			let asset_id = Self::convert_asset_id(rid);
			//check if rid already registered.
			if chainbridge::AssetIdToResourceMap::<T>::get(asset_id).is_some() {
				return Err(chainbridge::Error::<T>::ResourceAlreadyRegistered.into())
			}

			T::AssetManager::create(
				asset_id,
				chainbridge::Pallet::<T>::account_id(),
				true,
				BalanceOf::<T>::one().unique_saturated_into(),
			)?;
			chainbridge::AssetIdToResourceMap::<T>::insert(asset_id, rid);
			Self::deposit_event(Event::<T>::AssetRegistered(rid));
			Ok(())
		}

		/// Mints Asset into Recipient's Account
		/// Only Relayers can call it.
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `destination_add`: Recipient's Account
		/// * `amount`: Amount to be minted in Recipient's Account
		/// * `rid`: Resource ID
		#[allow(clippy::unnecessary_lazy_evaluations)]
		#[pallet::weight((195_000_000).saturating_add(T::DbWeight::get().writes(2 as Weight)))]
		pub fn mint_asset(
			origin: OriginFor<T>,
			destination_add: Vec<u8>,
			amount: u128,
			rid: ResourceId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(
				chainbridge::Pallet::<T>::account_id() == sender,
				Error::<T>::MinterMustBeRelayer
			);

			let destination_acc = T::AccountId::decode(&mut &destination_add[..])
				.map_err(|_| Error::<T>::DestinationAddressNotValid)?;

			let amount = Self::convert_18dec_to_12dec(amount)
				.ok_or_else(|| Error::<T>::DivisionUnderflow)?;

			let asset_id = Self::convert_asset_id(rid);
			if let Some(rid_present) = chainbridge::AssetIdToResourceMap::<T>::get(asset_id) {
				ensure!(rid_present == rid, chainbridge::Error::<T>::ResourceDoesNotExist)
			};
			T::AssetManager::mint_into(
				asset_id,
				&destination_acc,
				amount.saturated_into::<u128>(),
			)?;
			Self::deposit_event(Event::<T>::AssetDeposited(destination_acc, rid, amount));
			Ok(())
		}

		/// Set Bridge Status
		#[pallet::weight((195_000_000).saturating_add(T::DbWeight::get().writes(2 as Weight)))]
		pub fn set_bridge_status(origin: OriginFor<T>, status: bool) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<BridgeDeactivated<T>>::put(status);
			Self::deposit_event(Event::<T>::BridgeStatusUpdated(status));
			Ok(())
		}

		/// Set Block Delay
		#[pallet::weight(T::DbWeight::get().writes(2 as Weight))]
		pub fn set_block_delay(
			origin: OriginFor<T>,
			no_of_blocks: T::BlockNumber,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<WithdrawalExecutionBlockDiff<T>>::put(no_of_blocks);
			Self::deposit_event(Event::<T>::BlocksDelayUpdated(no_of_blocks));
			Ok(())
		}

		/// Transfers Asset to Destination Chain.
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `chain_id`: Asset's native chain
		/// * `contract_add`: Asset's actual address at native chain
		/// * `amount`: Amount to be burned and transferred from Sender's Account
		/// * `recipient`: recipient
		#[pallet::weight(T::WeightInfo::withdraw(1, 1))]
		pub fn withdraw(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			contract_add: H160,
			amount: BalanceOf<T>,
			recipient: H160,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;
			ensure!(
				<AllowlistedToken<T>>::get().contains(&contract_add),
				Error::<T>::TokenNotAllowlisted
			);
			ensure!(
				chainbridge::Pallet::<T>::chain_allowlisted(chain_id),
				Error::<T>::ChainIsNotAllowlisted
			);
			ensure!(!<BridgeDeactivated<T>>::get(), Error::<T>::BridgeDeactivated);
			let rid = chainbridge::derive_resource_id(chain_id, &contract_add.0);
			ensure!(
				T::AssetManager::reducible_balance(Self::convert_asset_id(rid), &sender, true) >=
					amount.saturated_into::<u128>(),
				Error::<T>::NotEnoughBalance
			);
			ensure!(
				<PendingWithdrawals<T>>::get(<frame_system::Pallet<T>>::block_number()).len() <
					WithdrawalLimit::get().try_into().map_err(|_| Error::<T>::ConversionIssue)?,
				Error::<T>::WithdrawalLimitReached
			);
			let fee = Self::fee_calculation(chain_id, amount)?;

			T::Currency::transfer(
				&sender,
				&chainbridge::Pallet::<T>::account_id(),
				fee,
				ExistenceRequirement::KeepAlive,
			)?;
			T::AssetManager::burn_from(
				Self::convert_asset_id(rid),
				&sender,
				amount.saturated_into::<u128>(),
			)?;

			let pending_withdrawal = PendingWithdrawal { chain_id, rid, recipient, amount };
			<PendingWithdrawals<T>>::try_mutate(
				<frame_system::Pallet<T>>::block_number(),
				|withdrawals| {
					withdrawals.try_push(pending_withdrawal)?;
					Ok(())
				},
			)
			.map_err(|()| Error::<T>::WithdrawalLimitReached)?;
			Self::deposit_event(Event::<T>::AssetWithdrawn(contract_add, rid, amount));
			Ok(())
		}

		/// Updates fee for given Chain id.
		///
		/// # Parameters
		///
		/// * `origin`: `Asset` owner
		/// * `chain_id`: Asset's native chain
		/// * `min_fee`: Minimum fee to be charged to transfer Asset to different.
		/// * `fee_scale`: Scale to find fee depending on amount.
		#[pallet::weight(T::WeightInfo::update_fee(1, 1))]
		pub fn update_fee(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			min_fee: BalanceOf<T>,
			fee_scale: u32,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<BridgeFee<T>>::insert(chain_id, (min_fee, fee_scale));
			Self::deposit_event(Event::<T>::FeeUpdated(chain_id, min_fee));
			Ok(())
		}

		/// Allowlists Token
		#[pallet::weight((195_000_000).saturating_add(T::DbWeight::get().writes(1 as Weight)))]
		pub fn allowlist_token(origin: OriginFor<T>, token_add: H160) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<AllowlistedToken<T>>::try_mutate(|allowlisted_tokens| {
				allowlisted_tokens
					.try_insert(token_add)
					.map_err(|_| Error::<T>::AllowlistedTokenLimitReached)?;
				Self::deposit_event(Event::<T>::AllowlistedTokenAdded(token_add));
				Ok(())
			})
		}

		/// Remove allowlisted tokens
		#[pallet::weight((195_000_000).saturating_add(T::DbWeight::get().writes(1 as Weight)))]
		pub fn remove_allowlisted_token(origin: OriginFor<T>, token_add: H160) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<AllowlistedToken<T>>::try_mutate(|allowlisted_tokens| {
				allowlisted_tokens.remove(&token_add);
				Self::deposit_event(Event::<T>::AllowlistedTokenRemoved(token_add));
				Ok(())
			})
		}
	}

	impl<T: Config> Pallet<T> {
		fn convert_balance_to_eth_type(balance: BalanceOf<T>) -> U256 {
			let balance: u128 = balance.unique_saturated_into();
			U256::from(balance).saturating_mul(U256::from(1000000u128))
		}

		fn fee_calculation(
			bridge_id: BridgeChainId,
			amount: BalanceOf<T>,
		) -> Result<BalanceOf<T>, DispatchError> {
			let (min_fee, fee_scale) = Self::get_bridge_fee(bridge_id);
			let fee_estimated: u128 =
				amount.saturating_mul(fee_scale.into()).unique_saturated_into();
			match fee_estimated.checked_div(1000_u128) {
				Some(fee_estimated) => {
					let fee_estimated = fee_estimated.saturated_into();
					if fee_estimated > min_fee {
						Ok(fee_estimated)
					} else {
						Ok(min_fee)
					}
				},
				None => Err(Error::<T>::DivisionOverflow.into()),
			}
		}

		fn convert_18dec_to_12dec(balance: u128) -> Option<u128> {
			balance.checked_div(1000000u128)
		}

		pub fn convert_asset_id(token: ResourceId) -> u128 {
			let mut temp = [0u8; 16];
			temp.copy_from_slice(&token[0..16]);
			//temp.copy_fro	m_slice(token.as_fixed_bytes().as_ref());
			u128::from_le_bytes(temp)
		}

		pub fn account_balances(assets: Vec<u128>, account_id: T::AccountId) -> Vec<u128> {
			assets
				.iter()
				.map(|asset| {
					<T as Config>::AssetManager::balance(*asset, &account_id).saturated_into()
				})
				.collect()
		}

		#[cfg(feature = "runtime-benchmarks")]
		pub fn register_asset(rid: ResourceId) {
			T::AssetManager::create(
				Self::convert_asset_id(rid),
				chainbridge::Pallet::<T>::account_id(),
				true,
				BalanceOf::<T>::one().unique_saturated_into(),
			)
			.expect("Asset not Registered");
		}

		#[cfg(feature = "runtime-benchmarks")]
		pub fn mint_token(account: T::AccountId, rid: ResourceId, amount: u128) {
			T::AssetManager::mint_into(Pallet::<T>::convert_asset_id(rid), &account, amount);
		}
	}
}
