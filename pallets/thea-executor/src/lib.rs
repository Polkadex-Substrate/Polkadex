// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! # Thea executor pallet.
//!
//! Pallet performs operations with balance (e.g. withdraw, claim deposit and set withdraw fee).

#![cfg_attr(not(feature = "std"), no_std)]

extern crate core;

use frame_support::pallet_prelude::Weight;
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub trait TheaExecutorWeightInfo {
	fn set_withdrawal_fee(_r: u32) -> Weight;
	fn update_asset_metadata(_r: u32) -> Weight;
	fn withdraw(r: u32) -> Weight;
	fn parachain_withdraw(_r: u32) -> Weight;
	fn evm_withdraw(_r: u32) -> Weight;
	fn on_initialize(x: u32, y: u32) -> Weight;
	fn burn_native_tokens() -> Weight;
	fn claim_deposit(_r: u32) -> Weight;
}

#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::SaturatedConversion,
		traits::{
			fungible::Mutate,
			fungibles::Inspect,
			tokens::{Fortitude, Precision, Preservation},
		},
		transactional,
	};
	use frame_system::pallet_prelude::*;
	use pallet_asset_conversion::Swap;
	use polkadex_primitives::{AssetId, Resolver};
	use sp_core::{H160, H256};
	use sp_runtime::{traits::AccountIdConversion, Saturating};
	use sp_std::vec::Vec;
	use thea_primitives::types::NewWithdraw;
	use thea_primitives::{
		types::{AssetMetadata, Deposit},
		Network, TheaBenchmarkHelper, TheaIncomingExecutor, TheaOutgoingExecutor, NATIVE_NETWORK,
	};
	use xcm::VersionedMultiLocation;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_asset_conversion::Config {
		/// Because this pallet emits events, it depends on the Runtime's definition of an
		/// event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Balances Pallet
		type Currency: frame_support::traits::tokens::fungible::Mutate<Self::AccountId>
			+ frame_support::traits::tokens::fungible::Inspect<Self::AccountId>;
		/// Assets Pallet
		type Assets: frame_support::traits::tokens::fungibles::Mutate<Self::AccountId>
			+ frame_support::traits::tokens::fungibles::Create<Self::AccountId>
			+ frame_support::traits::tokens::fungibles::Inspect<Self::AccountId>;
		/// Asset Id
		type AssetId: Member
			+ Parameter
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ Into<<<Self as pallet::Config>::Assets as Inspect<Self::AccountId>>::AssetId>
			+ From<u128>;
		type MultiAssetIdAdapter: From<AssetId>
			+ Into<<Self as pallet_asset_conversion::Config>::MultiAssetId>;

		type AssetBalanceAdapter: Into<<Self as pallet_asset_conversion::Config>::AssetBalance>
			+ Copy
			+ From<<Self as pallet_asset_conversion::Config>::AssetBalance>
			+ From<u128>
			+ Into<u128>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Something that executes the payload
		type Executor: thea_primitives::TheaOutgoingExecutor;
		/// Native Asset Id
		type NativeAssetId: Get<<Self as pallet::Config>::AssetId>;
		/// Thea PalletId
		#[pallet::constant]
		type TheaPalletId: Get<frame_support::PalletId>;

		type Swap: pallet_asset_conversion::Swap<
			Self::AccountId,
			u128,
			polkadex_primitives::AssetId,
		>;
		/// Total Withdrawals
		#[pallet::constant]
		type WithdrawalSize: Get<u32>;
		/// Existential Deposit
		#[pallet::constant]
		type ExistentialDeposit: Get<u128>;
		/// Para Id
		type ParaId: Get<u32>;
		/// Governance Origin
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Type representing the weight of this pallet
		type TheaExecWeightInfo: TheaExecutorWeightInfo;
	}

	/// Nonce used to generate randomness
	#[pallet::storage]
	#[pallet::getter(fn randomness_nonce)]
	pub(super) type RandomnessNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_withdrawals)]
	pub(super) type PendingWithdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, Vec<NewWithdraw>, ValueQuery>;

	/// Withdrawal Fees for each network
	#[pallet::storage]
	#[pallet::getter(fn witdrawal_fees)]
	pub(super) type WithdrawalFees<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u128, OptionQuery>;

	/// Withdrawal batches ready for signing
	#[pallet::storage]
	#[pallet::getter(fn ready_withdrawals)]
	pub(super) type ReadyWithdrawals<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		BlockNumberFor<T>,
		Blake2_128Concat,
		Network,
		Vec<NewWithdraw>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_approved_deposits)]
	pub(super) type ApprovedDeposits<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Deposit<T::AccountId>>, ValueQuery>;

	/// Stores the metadata ( asset_id => Metadata )
	#[pallet::storage]
	#[pallet::getter(fn asset_metadata)]
	pub type Metadata<T: Config> = StorageMap<_, Identity, u128, AssetMetadata, OptionQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Metadata set ( config )
		AssetMetadataSet(AssetMetadata),
		/// Deposit Approved event ( Network, recipient, asset_id, amount, id))
		DepositApproved(Network, T::AccountId, u128, u128, Vec<u8>),
		/// Deposit claimed event ( recipient, asset id, amount, id )
		DepositClaimed(T::AccountId, u128, u128, Vec<u8>),
		/// Deposit failed event ( network, encoded deposit)
		DepositFailed(Network, Vec<u8>),
		/// Withdrawal Queued ( network, from, beneficiary, assetId, amount, id )
		WithdrawalQueued(Network, T::AccountId, Vec<u8>, u128, u128, Vec<u8>),
		/// Withdrawal Ready (Network id )
		WithdrawalReady(Network),
		/// Withdrawal Failed ( Network ,Vec<Withdrawal>)
		WithdrawalFailed(Network, Vec<NewWithdraw>),
		/// Thea Public Key Updated ( network, new session id )
		TheaKeyUpdated(Network, u32),
		/// Withdrawal Fee Set (NetworkId, Amount)
		WithdrawalFeeSet(u8, u128),
		/// Native Token Burn event
		NativeTokenBurned(T::AccountId, u128),
		/// Withdrawal Sent (Network, Withdrawal Id,Batch Outgoing Nonce, Withdrawal Index)
		WithdrawalSent(Network, Vec<u8>, u64, u8),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Invalid decimal configuration
		InvalidDecimal,
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
		/// Failed To Decode
		FailedToDecode,
		/// Beneficiary Too Long
		BeneficiaryTooLong,
		/// Withdrawal Not Allowed
		WithdrawalNotAllowed,
		/// Withdrawal Fee Config Not Found
		WithdrawalFeeConfigNotFound,
		/// Asset Not Registered
		AssetNotRegistered,
		/// Fee Asset Not Registered
		FeeAssetNotRegistered,
		/// Amount cannot be Zero
		AmountCannotBeZero,
		/// Failed To Handle Parachain Deposit
		FailedToHandleParachainDeposit,
		/// Token Type Not Handled
		TokenTypeNotHandled,
		/// Bounded Vector Overflow
		BoundedVectorOverflow,
		/// Bounded vector not present
		BoundedVectorNotPresent,
		/// No Approved Deposit
		NoApprovedDeposit,
		/// Wrong network
		WrongNetwork,
		/// Not able to get price for fee swap
		CannotSwapForFees,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_no: BlockNumberFor<T>) -> Weight {
			let pending_withdrawals =
				<ReadyWithdrawals<T>>::iter_prefix(block_no.saturating_sub(1u8.into()));
			let mut withdrawal_len = 0;
			let mut network_len = 0;
			for (network_id, withdrawals) in pending_withdrawals {
				withdrawal_len += withdrawals.len();
				let batch_nonce = T::Executor::get_outgoing_nonce(network_id);
				for (index, withdrawal) in withdrawals.iter().enumerate() {
					Self::deposit_event(Event::<T>::WithdrawalSent(
						network_id,
						withdrawal.id.clone(),
						batch_nonce,
						index as u8,
					));
				}
				// This is fine as this trait is not supposed to fail
				if T::Executor::execute_withdrawals(network_id, withdrawals.clone().encode())
					.is_err()
				{
					Self::deposit_event(Event::<T>::WithdrawalFailed(network_id, withdrawals))
				}
				network_len += 1;
			}
			T::TheaExecWeightInfo::on_initialize(network_len as u32, withdrawal_len as u32)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::withdraw(1))]
		#[transactional]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
			network: Network,
			pay_with_tokens: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			// Assumes the foreign chain can decode the given vector bytes as recipient
			Self::do_withdraw(
				user,
				asset_id,
				amount,
				beneficiary,
				None,
				None,
				pay_for_remaining,
				network,
				pay_with_tokens,
			)?;
			Ok(())
		}

		/// Add Token Config.
		///
		/// # Parameters
		///
		/// * `network_id`: Network Id.
		/// * `fee`: Withdrawal Fee.
		#[pallet::call_index(1)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::set_withdrawal_fee(1))]
		pub fn set_withdrawal_fee(
			origin: OriginFor<T>,
			network_id: u8,
			fee: u128,
		) -> DispatchResult {
			ensure_root(origin)?;
			<WithdrawalFees<T>>::insert(network_id, fee);
			Self::deposit_event(Event::<T>::WithdrawalFeeSet(network_id, fee));
			Ok(())
		}

		/// Withdraws to parachain networks in Polkadot
		#[pallet::call_index(2)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::parachain_withdraw(1))]
		pub fn parachain_withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: sp_std::boxed::Box<VersionedMultiLocation>,
			fee_asset_id: Option<u128>,
			fee_amount: Option<u128>,
			pay_for_remaining: bool,
			pay_with_tokens: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			let network = 1;
			Self::do_withdraw(
				user,
				asset_id,
				amount,
				beneficiary.encode(),
				fee_asset_id,
				fee_amount,
				pay_for_remaining,
				network,
				pay_with_tokens,
			)?;
			Ok(())
		}

		/// Update the Decimal metadata for an asset
		///
		/// # Parameters
		///
		/// * `asset_id`: Asset Id.
		/// * `metadata`: AssetMetadata.
		#[pallet::call_index(3)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::update_asset_metadata(1))]
		pub fn update_asset_metadata(
			origin: OriginFor<T>,
			asset_id: u128,
			decimal: u8,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let metadata = AssetMetadata::new(decimal).ok_or(Error::<T>::InvalidDecimal)?;
			<Metadata<T>>::insert(asset_id, metadata);
			Self::deposit_event(Event::<T>::AssetMetadataSet(metadata));
			Ok(())
		}

		/// Burn Native tokens of an account
		///
		/// # Parameters
		///
		/// * `who`: AccountId
		/// * `amount`: Amount of native tokens to burn.
		#[pallet::call_index(4)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::burn_native_tokens())]
		pub fn burn_native_tokens(
			origin: OriginFor<T>,
			who: T::AccountId,
			amount: u128,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let burned_amt = <T as Config>::Currency::burn_from(
				&who,
				amount.saturated_into(),
				Precision::BestEffort,
				Fortitude::Force,
			)?;
			Self::deposit_event(Event::<T>::NativeTokenBurned(who, burned_amt.saturated_into()));
			Ok(())
		}

		/// Withdraws to Ethereum network
		///
		/// # Parameters
		///
		/// * `asset_id`: Asset Id.
		/// * `amount`: Amount of tokens to withdraw.
		/// * `beneficiary`: Beneficiary address.
		/// * `pay_for_remaining`: Pay for remaining pending withdrawals.
		/// * `pay_with_tokens`: Pay with withdrawing tokens.
		#[pallet::call_index(5)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::evm_withdraw(1))]
		pub fn evm_withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: H160,
			network: Network,
			pay_for_remaining: bool,
			pay_with_tokens: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			Self::do_withdraw(
				user,
				asset_id,
				amount,
				beneficiary.encode(),
				None,
				None,
				pay_for_remaining,
				network,
				pay_with_tokens,
			)?;
			Ok(())
		}

		/// Manually claim an approved deposit.
		///
		/// # Parameters
		///
		/// * `origin`: User.
		/// * `num_deposits`: Number of deposits to claim from available deposits,
		/// (it's used to parametrise the weight of this extrinsic).
		#[pallet::call_index(6)]
		#[pallet::weight(< T as Config >::TheaExecWeightInfo::claim_deposit(1))]
		#[transactional]
		pub fn claim_deposit(
			origin: OriginFor<T>,
			num_deposits: u32,
			user: T::AccountId,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let mut deposits = <ApprovedDeposits<T>>::get(&user);
			let length: u32 = deposits.len().saturated_into();
			let length: u32 = if length <= num_deposits { length } else { num_deposits };
			for _ in 0..length {
				if let Some(deposit) = deposits.pop() {
					if let Err(err) = Self::execute_deposit(deposit.clone()) {
						deposits.push(deposit);
						// Save it back on failure
						<ApprovedDeposits<T>>::insert(&user, deposits.clone());
						return Err(err);
					}
				} else {
					break;
				}
			}

			if !deposits.is_empty() {
				// If pending deposits are available, save it back
				<ApprovedDeposits<T>>::insert(&user, deposits)
			} else {
				<ApprovedDeposits<T>>::remove(&user);
			}

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Generates a new random id for withdrawals
		fn new_random_id() -> Vec<u8> {
			let mut nonce = <RandomnessNonce<T>>::get();
			nonce = nonce.wrapping_add(1);
			<RandomnessNonce<T>>::put(nonce);
			let entropy = sp_io::hashing::blake2_256(&(NATIVE_NETWORK, nonce).encode());
			let entropy = H256::from_slice(&entropy).0[..10].to_vec();
			entropy.to_vec()
		}
		pub fn thea_account() -> T::AccountId {
			T::TheaPalletId::get().into_account_truncating()
		}

		#[transactional]
		pub fn do_withdraw(
			user: T::AccountId,
			asset_id: u128,
			mut amount: u128,
			beneficiary: Vec<u8>,
			fee_asset_id: Option<u128>,
			fee_amount: Option<u128>,
			pay_for_remaining: bool,
			network: Network,
			pay_with_tokens: bool,
		) -> Result<(), DispatchError> {
			ensure!(beneficiary.len() <= 1000, Error::<T>::BeneficiaryTooLong);
			ensure!(network != 0, Error::<T>::WrongNetwork);
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(network);
			let metadata = <Metadata<T>>::get(asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			if let Some(fee_asset_id) = fee_asset_id {
				let _metadata = <crate::pallet::Metadata<T>>::get(fee_asset_id)
					.ok_or(Error::<T>::FeeAssetNotRegistered)?;
			}
			ensure!(
				pending_withdrawals.len() < T::WithdrawalSize::get() as usize,
				Error::<T>::WithdrawalNotAllowed
			);

			let mut total_fees =
				<WithdrawalFees<T>>::get(network).ok_or(Error::<T>::WithdrawalFeeConfigNotFound)?;

			if pay_for_remaining {
				// User is ready to pay for remaining pending withdrawal for quick withdrawal
				let extra_withdrawals_available =
					T::WithdrawalSize::get().saturating_sub(pending_withdrawals.len() as u32);
				total_fees =
					total_fees.saturating_add(total_fees.saturating_mul(
						extra_withdrawals_available.saturating_sub(1).saturated_into(),
					))
			}

			if pay_with_tokens {
				// User wants to pay with withdrawing tokens.
				let path = sp_std::vec![
					polkadex_primitives::AssetId::Asset(asset_id),
					polkadex_primitives::AssetId::Polkadex
				];
				let token_taken = T::Swap::swap_tokens_for_exact_tokens(
					user.clone(),
					path,
					total_fees.saturated_into(),
					None,
					Self::thea_account(),
					false,
				)?;
				amount = amount.saturating_sub(token_taken.saturated_into());
				ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			} else {
				// Pay the fees
				<T as Config>::Currency::transfer(
					&user,
					&Self::thea_account(),
					total_fees.saturated_into(),
					Preservation::Preserve,
				)?;
			}

			// Withdraw assets
			Self::resolver_withdraw(asset_id.into(), amount, &user, Self::thea_account())?;

			if let (Some(fee_asset_id), Some(fee_amount)) = (fee_asset_id, fee_amount) {
				Self::resolver_withdraw(
					fee_asset_id.into(),
					fee_amount,
					&user,
					Self::thea_account(),
				)?;
			}

			let mut withdraw = NewWithdraw {
				id: Self::new_random_id(),
				asset_id,
				amount,
				destination: beneficiary.clone(),
				fee_asset_id,
				fee_amount,
				is_blocked: false,
				extra: Vec::new(),
			};

			Self::deposit_event(Event::<T>::WithdrawalQueued(
				network,
				user,
				beneficiary,
				asset_id,
				amount,
				withdraw.id.clone(),
			));

			// Convert back to origin decimals
			withdraw.amount = metadata.convert_from_native_decimals(amount);

			if let (Some(fee_asset_id), Some(fee_amount)) = (fee_asset_id, fee_amount) {
				let metadata = <crate::pallet::Metadata<T>>::get(fee_asset_id)
					.ok_or(Error::<T>::FeeAssetNotRegistered)?;
				withdraw.fee_amount = Some(metadata.convert_from_native_decimals(fee_amount));
			}

			pending_withdrawals.push(withdraw);

			if (pending_withdrawals.len() >= T::WithdrawalSize::get() as usize) || pay_for_remaining
			{
				// If it is full then we move it to ready queue and update withdrawal nonce
				<ReadyWithdrawals<T>>::insert(
					<frame_system::Pallet<T>>::block_number(), //Block No
					network,
					pending_withdrawals.clone(),
				);
				Self::deposit_event(Event::<T>::WithdrawalReady(network));
				pending_withdrawals = Vec::default();
			}
			<PendingWithdrawals<T>>::insert(network, pending_withdrawals);
			Ok(())
		}

		#[transactional]
		pub fn do_deposit(network: Network, payload: &[u8]) -> Result<(), DispatchError> {
			let deposits: Vec<Deposit<T::AccountId>> =
				Decode::decode(&mut &payload[..]).map_err(|_| Error::<T>::FailedToDecode)?;
			for deposit in deposits {
				// Execute Deposit
				Self::execute_deposit(deposit.clone())?;
				Self::deposit_event(Event::<T>::DepositApproved(
					network,
					deposit.recipient,
					deposit.asset_id,
					deposit.amount,
					deposit.id,
				))
			}
			Ok(())
		}

		#[transactional]
		pub fn execute_deposit(deposit: Deposit<T::AccountId>) -> Result<(), DispatchError> {
			// Get the metadata
			let metadata =
				<Metadata<T>>::get(deposit.asset_id).ok_or(Error::<T>::AssetNotRegistered)?;
			let deposit_amount = deposit.amount_in_native_decimals(metadata); // Convert the decimals configured in metadata

			if !frame_system::Pallet::<T>::account_exists(&deposit.recipient) {
				let path = sp_std::vec![
					polkadex_primitives::AssetId::Asset(deposit.asset_id),
					polkadex_primitives::AssetId::Polkadex
				];
				let amount_out: T::AssetBalanceAdapter = T::ExistentialDeposit::get().into();
				Self::resolve_mint(&Self::thea_account(), deposit.asset_id.into(), deposit_amount)?;

				// If swap doesn't work then it will in the system account - thea_account()
				if let Ok(fee_amount) = T::Swap::swap_tokens_for_exact_tokens(
					Self::thea_account(),
					path,
					amount_out.into(),
					Some(deposit_amount),
					deposit.recipient.clone(),
					true,
				) {
					Self::resolve_transfer(
						deposit.asset_id.into(),
						&Self::thea_account(),
						&deposit.recipient,
						deposit_amount.saturating_sub(fee_amount),
					)?;
				}
			} else {
				Self::resolver_deposit(
					deposit.asset_id.into(),
					deposit_amount,
					&deposit.recipient,
					Self::thea_account(),
					1u128,
					Self::thea_account(),
				)?;
			}

			// Emit event
			Self::deposit_event(Event::<T>::DepositClaimed(
				deposit.recipient.clone(),
				deposit.asset_id,
				deposit.amount_in_native_decimals(metadata),
				deposit.id,
			));
			Ok(())
		}
	}

	impl<T: Config> TheaIncomingExecutor for Pallet<T> {
		fn execute_deposits(network: Network, deposits: Vec<u8>) {
			if let Err(error) = Self::do_deposit(network, &deposits) {
				Self::deposit_event(Event::<T>::DepositFailed(network, deposits));
				log::error!(target:"thea","Deposit Failed : {:?}", error);
			}
		}
	}

	// Implement this trait for handing deposits and withdrawals
	impl<T: Config>
		polkadex_primitives::assets::Resolver<
			T::AccountId,
			<T as pallet::Config>::Currency,
			<T as pallet::Config>::Assets,
			<T as pallet::Config>::AssetId,
			<T as Config>::NativeAssetId,
		> for Pallet<T>
	{
	}

	impl<T: Config> TheaBenchmarkHelper for Pallet<T> {
		fn set_metadata(asset_id: AssetId) {
			let metadata = AssetMetadata::new(12).unwrap();
			if let AssetId::Asset(asset) = asset_id {
				<Metadata<T>>::insert(asset, metadata);
			}
		}
	}
}
