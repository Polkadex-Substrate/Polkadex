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
#![deny(unused_crate_dependencies)]
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

use chainbridge::{BridgeChainId, ResourceId};
use frame_support::{
	dispatch::fmt::Debug,
	fail, log,
	pallet_prelude::*,
	traits::{
		fungibles::Transfer,
		tokens::{
			fungibles::{Create, Inspect, Mutate},
			DepositConsequence, WithdrawConsequence,
		},
		Currency, ExistenceRequirement, ReservableCurrency,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use sp_core::{H160, U256};
use sp_io::hashing::keccak_256;
use sp_runtime::{
	traits::{One, Saturating, UniqueSaturatedInto},
	BoundedBTreeSet, SaturatedConversion,
};
use sp_std::{vec, vec::Vec};

pub trait WeightInfo {
	fn create_asset(_b: u32) -> Weight;
	fn create_thea_asset() -> Weight;
	fn create_parachain_asset() -> Weight;
	fn mint_asset(_b: u32) -> Weight;
	fn set_bridge_status() -> Weight;
	fn set_block_delay() -> Weight;
	fn update_fee(_m: u32, f: u32) -> Weight;
	fn withdraw(_b: u32, c: u32) -> Weight;
	fn allowlist_token(b: u32) -> Weight;
	fn remove_allowlisted_token(b: u32) -> Weight;
	fn add_precision(_b: u32) -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

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

	#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
	pub enum PrecisionType {
		LowPrecision(u128),
		HighPrecision(u128),
		SamePrecision,
	}

	impl Default for PrecisionType {
		fn default() -> Self {
			Self::SamePrecision
		}
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
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Asset Manager
		type AssetManager: Create<<Self as frame_system::Config>::AccountId>
			+ Mutate<<Self as frame_system::Config>::AccountId, Balance = u128, AssetId = u128>
			+ Inspect<<Self as frame_system::Config>::AccountId>
			+ Transfer<<Self as frame_system::Config>::AccountId>;

		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		//PDEX asset id
		#[pallet::constant]
		type NativeCurrencyId: Get<u128>;

		/// Treasury PalletId
		#[pallet::constant]
		type TreasuryPalletId: Get<PalletId>;

		/// Parachain Network Id
		#[pallet::constant]
		type ParachainNetworkId: Get<u8>;

		/// PDEX Token Holder Account
		type PDEXHolderAccount: Get<Self::AccountId>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
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

	/// AssetId Precision
	#[pallet::storage]
	#[pallet::getter(fn get_precision)]
	pub(super) type AssetPrecision<T: Config> =
		StorageMap<_, Blake2_128Concat, ResourceId, PrecisionType, ValueQuery>;

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
		FungibleTransferFailed(ResourceId),
		/// This token got allowlisted
		AllowlistedTokenAdded(H160),
		/// This token got removed from Allowlisted Tokens
		AllowlistedTokenRemoved(H160),
		/// Thea Asset has been register
		TheaAssetCreated(u128),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Migration is not operational yet
		NotOperational,
		/// MinterIsNotValid
		MinterIsNotValid,
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
		/// Amount for minting or burning cannot be Zero
		AmountCannotBeZero,
		/// Thea Asset has not been registered
		AssetNotRegistered,
		// Identifier length provided is wrong
		IdentifierLengthMismatch,
		//when trying to burn PDEX asset
		CannotBurnNativeAsset,
		//when trying to mint PDEX asset
		CannotMintNativeAsset,
		//when cannot transfer PDEX asset
		NativeAssetTransferFailed,
		/// ReservedParachainNetworkId
		ReservedParachainNetworkId,
		/// AssetId Abstract Not Handled
		AssetIdAbstractNotHandled,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// On Initialize
		fn on_initialize(n: T::BlockNumber) -> Weight {
			let mut failed_withdrawal: BoundedVec<
				PendingWithdrawal<BalanceOf<T>>,
				WithdrawalLimit,
			> = BoundedVec::default();
			<PendingWithdrawals<T>>::mutate(n, |withdrawals| {
				while let Some(withdrawal) = withdrawals.pop() {
					if let Some(amount) =
						Self::convert_amount_for_foreign_chain(withdrawal.rid, withdrawal.amount)
					{
						if chainbridge::Pallet::<T>::transfer_fungible(
							withdrawal.chain_id,
							withdrawal.rid,
							withdrawal.recipient.0.to_vec(),
							amount,
						)
						.is_err()
						{
							if failed_withdrawal.try_push(withdrawal.clone()).is_err() {
								log::error!(target:"asset-handler", "Failed to push into Withdrawal");
							}
							Self::deposit_event(Event::<T>::FungibleTransferFailed(withdrawal.rid));
						}
					} else {
						log::error!(target:"asset-handler", "Division Overflow");
					}
				}
			});
			<PendingWithdrawals<T>>::insert(n, failed_withdrawal);
			// TODO: Benchmark on initialize
			T::DbWeight::get().writes(5).saturating_add(T::DbWeight::get().reads(5))
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
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::create_asset(1))]
		pub fn create_asset(
			origin: OriginFor<T>,
			chain_id: BridgeChainId,
			contract_add: H160,
			precision_type: PrecisionType,
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
			<AssetPrecision<T>>::insert(rid, precision_type);
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
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::mint_asset(1))]
		pub fn mint_asset(
			origin: OriginFor<T>,
			destination_add: Vec<u8>,
			amount: u128,
			rid: ResourceId,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(chainbridge::Pallet::<T>::account_id() == sender, Error::<T>::MinterIsNotValid);

			let destination_acc = T::AccountId::decode(&mut &destination_add[..])
				.map_err(|_| Error::<T>::DestinationAddressNotValid)?;

			let amount = Self::convert_amount_for_native_chain(rid, amount)
				.ok_or_else(|| Error::<T>::DivisionUnderflow)?;

			let asset_id = Self::convert_asset_id(rid);
			if let Some(rid_present) = chainbridge::AssetIdToResourceMap::<T>::get(asset_id) {
				ensure!(rid_present == rid, chainbridge::Error::<T>::ResourceDoesNotExist)
			};
			//
			T::AssetManager::mint_into(
				asset_id,
				&destination_acc,
				amount.saturated_into::<u128>(),
			)?;
			Self::deposit_event(Event::<T>::AssetDeposited(destination_acc, rid, amount));
			Ok(())
		}

		/// Set Bridge Status
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::set_bridge_status())]
		pub fn set_bridge_status(origin: OriginFor<T>, status: bool) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<BridgeDeactivated<T>>::put(status);
			Self::deposit_event(Event::<T>::BridgeStatusUpdated(status));
			Ok(())
		}

		/// Set Block Delay
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::set_block_delay())]
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
		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::withdraw(1, 1))]
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
			let withdrawal_execution_block = <frame_system::Pallet<T>>::block_number()
				.saturated_into::<u32>()
				.saturating_add(<WithdrawalExecutionBlockDiff<T>>::get().saturated_into::<u32>());
			<PendingWithdrawals<T>>::try_mutate(
				withdrawal_execution_block.saturated_into::<T::BlockNumber>(),
				|withdrawals| {
					if withdrawals.try_push(pending_withdrawal).is_err() {
						return Err(())
					}
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
		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::update_fee(1, 1))]
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
		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::allowlist_token(1))]
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
		#[pallet::call_index(9)]
		#[pallet::weight(<T as Config>::WeightInfo::remove_allowlisted_token(1))]
		pub fn remove_allowlisted_token(origin: OriginFor<T>, token_add: H160) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<AllowlistedToken<T>>::try_mutate(|allowlisted_tokens| {
				allowlisted_tokens.remove(&token_add);
				Self::deposit_event(Event::<T>::AllowlistedTokenRemoved(token_add));
				Ok(())
			})
		}

		/// Remove allowlisted tokens
		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::add_precision(1))]
		pub fn add_precision(
			origin: OriginFor<T>,
			rid: ResourceId,
			precision_type: PrecisionType,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			<AssetPrecision<T>>::insert(rid, precision_type);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn convert_amount_for_foreign_chain(
			rid: ResourceId,
			balance: BalanceOf<T>,
		) -> Option<U256> {
			let balance: u128 = balance.saturated_into();
			match <AssetPrecision<T>>::get(rid) {
				PrecisionType::LowPrecision(precision) =>
					U256::from(balance).checked_div(U256::from(precision)),
				PrecisionType::HighPrecision(precision) =>
					Some(U256::from(balance).saturating_mul(U256::from(precision))),
				PrecisionType::SamePrecision => Some(U256::from(balance)),
			}
		}

		pub fn convert_amount_for_native_chain(rid: ResourceId, amount: u128) -> Option<u128> {
			match <AssetPrecision<T>>::get(rid) {
				PrecisionType::LowPrecision(precision) => Some(amount.saturating_mul(precision)),
				PrecisionType::HighPrecision(precision) => amount.checked_div(precision),
				PrecisionType::SamePrecision => Some(amount),
			}
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

		/// converts `balance` from 18 decimal points to 12
		/// by dividing it by 1_000_000
		pub fn convert_18dec_to_12dec(balance: u128) -> Option<u128> {
			balance.checked_div(1000000u128)
		}

		pub fn convert_asset_id(token: ResourceId) -> u128 {
			let mut temp = [0u8; 16];
			temp.copy_from_slice(&token[0..16]);
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

		pub fn mint_thea_asset(
			asset_id: u128,
			recipient: T::AccountId,
			amount: u128,
		) -> Result<(), DispatchError> {
			if !T::AssetManager::asset_exists(asset_id) {
				T::AssetManager::create(asset_id, T::PDEXHolderAccount::get(), true, 1u128)?;
			}
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			T::AssetManager::mint_into(asset_id, &recipient, amount)?;
			Ok(())
		}

		/// Asset Handler for Withdraw Extrinsic
		/// # Parameters
		///
		/// * `asset_id`: Asset Id.
		/// * `who`: Asset Holder.
		/// * `amount`: Amount to be burned/locked.
		pub fn handle_asset(
			asset_id: u128,
			who: T::AccountId,
			amount: u128,
		) -> Result<(), DispatchError> {
			let polkadex_asset_id = T::NativeCurrencyId::get();
			if polkadex_asset_id == asset_id {
				Self::lock_pdex_asset(amount, who)
			} else {
				Self::burn_thea_asset(asset_id, who, amount)
			}
		}

		/// Asset Locker
		/// # Parameters
		///
		/// * `amount`: Amount to be locked.
		/// * `who`: Asset Holder.
		pub fn lock_pdex_asset(amount: u128, who: T::AccountId) -> DispatchResult {
			let polkadex_holder_account = T::PDEXHolderAccount::get();
			T::Currency::transfer(
				&who,
				&polkadex_holder_account,
				amount.saturated_into(),
				ExistenceRequirement::AllowDeath,
			)
		}

		pub fn burn_thea_asset(
			asset_id: u128,
			who: T::AccountId,
			amount: u128,
		) -> Result<(), DispatchError> {
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			T::AssetManager::burn_from(asset_id, &who, amount)?;
			Ok(())
		}

		pub fn get_asset_id(derived_asset_id: Vec<u8>) -> u128 {
			let derived_asset_id_hash = &keccak_256(derived_asset_id.as_ref())[0..16];
			let mut temp = [0u8; 16];
			temp.copy_from_slice(derived_asset_id_hash);
			u128::from_le_bytes(temp)
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
			T::AssetManager::mint_into(Pallet::<T>::convert_asset_id(rid), &account, amount)
				.unwrap();
		}
	}

	impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
		type AssetId = u128;
		type Balance = u128;

		fn total_issuance(asset: Self::AssetId) -> Self::Balance {
			// when asset is not polkadex
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::total_issuance(asset.saturated_into()).saturated_into()
			} else {
				T::Currency::total_issuance().saturated_into()
			}
		}

		fn active_issuance(asset: Self::AssetId) -> Self::Balance {
			T::AssetManager::active_issuance(asset)
		}

		fn minimum_balance(asset: Self::AssetId) -> Self::Balance {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::minimum_balance(asset.saturated_into()).saturated_into()
			} else {
				T::Currency::minimum_balance().saturated_into()
			}
		}

		fn balance(asset: Self::AssetId, who: &T::AccountId) -> Self::Balance {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::balance(asset.saturated_into(), who).saturated_into()
			} else {
				T::Currency::total_balance(who).saturated_into()
			}
		}

		fn reducible_balance(
			asset: Self::AssetId,
			who: &T::AccountId,
			keep_alive: bool,
		) -> Self::Balance {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::reducible_balance(asset.saturated_into(), who, keep_alive)
					.saturated_into()
			} else {
				T::Currency::free_balance(who).saturated_into()
			}
		}

		fn can_deposit(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
			mint: bool,
		) -> DepositConsequence {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::can_deposit(asset, who, amount.saturated_into(), mint)
			} else {
				// balance of native asset can always be increased
				DepositConsequence::Success
			}
		}

		fn can_withdraw(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> WithdrawConsequence<Self::Balance> {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::can_withdraw(asset.saturated_into(), who, amount.saturated_into())
			} else if T::Currency::free_balance(who) >= amount.saturated_into() {
				WithdrawConsequence::Success
			} else {
				// TODO: Need a better error mapping
				WithdrawConsequence::UnknownAsset
			}
		}

		fn asset_exists(asset: Self::AssetId) -> bool {
			T::AssetManager::asset_exists(asset)
		}
	}

	impl<T: Config> Transfer<T::AccountId> for Pallet<T> {
		fn transfer(
			asset: Self::AssetId,
			source: &T::AccountId,
			dest: &T::AccountId,
			amount: Self::Balance,
			keep_alive: bool,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::transfer(asset, source, dest, amount.saturated_into(), keep_alive)
					.map(|x| x.saturated_into())
			} else {
				let existence_requirement = if keep_alive {
					ExistenceRequirement::KeepAlive
				} else {
					ExistenceRequirement::AllowDeath
				};
				T::Currency::transfer(
					source,
					dest,
					amount.saturated_into(),
					existence_requirement,
				)?;
				Ok(amount)
			}
		}
	}

	impl<T: Config> Mutate<T::AccountId> for Pallet<T> {
		fn mint_into(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> DispatchResult {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::mint_into(asset, who, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				fail!(Error::<T>::CannotMintNativeAsset)
			}
		}

		fn burn_from(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::burn_from(asset, who, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				fail!(Error::<T>::CannotBurnNativeAsset)
			}
		}

		fn slash(
			asset: Self::AssetId,
			who: &T::AccountId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::slash(asset, who, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				let (_, balance) = T::Currency::slash(who, amount.saturated_into());
				Ok(balance.saturated_into())
			}
		}

		fn teleport(
			asset: Self::AssetId,
			source: &T::AccountId,
			dest: &T::AccountId,
			amount: Self::Balance,
		) -> Result<Self::Balance, DispatchError> {
			if asset != T::NativeCurrencyId::get() {
				T::AssetManager::teleport(asset, source, dest, amount.saturated_into())
					.map(|x| x.saturated_into())
			} else {
				T::Currency::transfer(
					source,
					dest,
					amount.saturated_into(),
					ExistenceRequirement::KeepAlive,
				)?;
				Ok(amount)
			}
		}
	}
}
