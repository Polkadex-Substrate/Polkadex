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

use frame_support::pallet_prelude::Weight;
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

pub trait WeightInfo {
	fn set_withdrawal_fee(_r: u32) -> Weight;
	fn update_asset_metadata(_r: u32) -> Weight;
	fn claim_deposit(r: u32) -> Weight;
	fn withdraw(r: u32) -> Weight;
	fn parachain_withdraw(_r: u32) -> Weight;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::SaturatedConversion,
		traits::{fungible::Mutate, fungibles::Inspect, tokens::Preservation},
	};
	use frame_system::pallet_prelude::*;
	use polkadex_primitives::{ Resolver};
	use sp_core::{H160, H256};
	use sp_runtime::{traits::AccountIdConversion, Saturating};
	use sp_std::vec::Vec;
	use thea_primitives::{ethereum::{EthereumOP, EtherumAction}, types::{AssetMetadata, Deposit, Withdraw}, Network, TheaIncomingExecutor, TheaOutgoingExecutor, NATIVE_NETWORK, PARACHAIN_NETWORK, ETHEREUM_NETWORK};
	use xcm::VersionedMultiLocation;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + pallet_ocex_lmp::Config {
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
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Something that executes the payload
		type Executor: thea_primitives::TheaOutgoingExecutor;
		/// Native Asset Id
		type NativeAssetId: Get<Self::AssetId>;
		/// Thea PalletId
		#[pallet::constant]
		type TheaPalletId: Get<frame_support::PalletId>;
		/// Total Withdrawals
		#[pallet::constant]
		type WithdrawalSize: Get<u32>;
		/// Para Id
		type ParaId: Get<u32>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	/// Nonce used to generate randomness
	#[pallet::storage]
	#[pallet::getter(fn randomness_nonce)]
	pub(super) type RandomnessNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn pending_withdrawals)]
	pub(super) type PendingWithdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, Vec<Withdraw>, ValueQuery>;

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
		Vec<Withdraw>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_approved_deposits)]
	pub(super) type ApprovedDeposits<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, Vec<Deposit<T::AccountId>>, ValueQuery>;

	/// Stores the metadata ( asset_id => Metadata )
	#[pallet::storage]
	#[pallet::getter(fn asset_metadata)]
	pub(super) type Metadata<T: Config> = StorageMap<_, Identity, u128, AssetMetadata, OptionQuery>;

	/// Map between Eth token contract and asset_id (u128)
	#[pallet::storage]
	#[pallet::getter(fn ethereum_asset_mapping)]
	pub(super) type EthereumAssetMapping<T: Config> =
		StorageMap<_, Identity, H160, u128, OptionQuery>;

	/// Reverse Map between asset_id (u128) and Eth token contract
	#[pallet::storage]
	#[pallet::getter(fn reverse_ethereum_asset_mapping)]
	pub(super) type EthereumAssetReverseMapping<T: Config> =
		StorageMap<_, Identity, u128, H160, OptionQuery>;

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
		/// Withdrawal Queued ( network, from, beneficiary, assetId, amount, id )
		WithdrawalQueued(Network, T::AccountId, Vec<u8>, u128, u128, Vec<u8>),
		/// Withdrawal Ready (Network id )
		WithdrawalReady(Network),
		// Thea Public Key Updated ( network, new session id )
		TheaKeyUpdated(Network, u32),
		/// Withdrawal Fee Set (NetworkId, Amount)
		WithdrawalFeeSet(u8, u128),
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_no: BlockNumberFor<T>) -> Weight {
			let pending_withdrawals =
				<ReadyWithdrawals<T>>::iter_prefix(block_no.saturating_sub(1u8.into()));
			for (network_id, withdrawal) in pending_withdrawals {
				// This is fine as this trait is not supposed to fail
				if T::Executor::execute_withdrawals(network_id, withdrawal.encode()).is_err() {
					log::error!("Error while executing withdrawals...");
				}
			}
			//TODO: Clean Storage
			Weight::default()
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatch able that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::withdraw(1))]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
			network: Network,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			// Assumes the foreign chain can decode the given vector bytes as recipient
			Self::do_withdraw(user, asset_id, amount, beneficiary, pay_for_remaining, network)?;
			Ok(())
		}

		/// Add Token Config.
		///
		/// # Parameters
		///
		/// * `network_id`: Network Id.
		/// * `fee`: Withdrawal Fee.
		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::set_withdrawal_fee(1))]
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
		#[pallet::call_index(3)]
		#[pallet::weight(<T as Config>::WeightInfo::parachain_withdraw(1))]
		pub fn parachain_withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: sp_std::boxed::Box<VersionedMultiLocation>,
			pay_for_remaining: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			let network = PARACHAIN_NETWORK;
			Self::do_withdraw(
				user,
				asset_id,
				amount,
				beneficiary.encode(),
				pay_for_remaining,
				network,
			)?;
			Ok(())
		}

		/// Update the Decimal metadata for an asset
		///
		/// # Parameters
		///
		/// * `asset_id`: Asset Id.
		/// * `metadata`: AssetMetadata.
		#[pallet::call_index(4)]
		#[pallet::weight(<T as Config>::WeightInfo::update_asset_metadata(1))]
		pub fn update_asset_metadata(
			origin: OriginFor<T>,
			asset_id: u128,
			decimal: u8,
		) -> DispatchResult {
			ensure_root(origin)?;
			let metadata = AssetMetadata::new(decimal).ok_or(Error::<T>::InvalidDecimal)?;
			<Metadata<T>>::insert(asset_id, metadata);
			Self::deposit_event(Event::<T>::AssetMetadataSet(metadata));
			Ok(())
		}


		/// Withdraws to Ethereum network
		#[pallet::call_index(5)]
		#[pallet::weight(<T as Config>::WeightInfo::parachain_withdraw(1))]
		pub fn ethereum_withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: H160,
			pay_for_remaining: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			let network = ETHEREUM_NETWORK;
			Self::do_withdraw(
				user,
				asset_id,
				amount,
				beneficiary.encode(),
				pay_for_remaining,
				network,
			)?;
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
			entropy.to_vec()
		}
		pub fn thea_account() -> T::AccountId {
			T::TheaPalletId::get().into_account_truncating()
		}

		pub fn do_withdraw(
			user: T::AccountId,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
			network: Network,
		) -> Result<(), DispatchError> {
			ensure!(beneficiary.len() <= 1000, Error::<T>::BeneficiaryTooLong);
			ensure!(network != 0, Error::<T>::WrongNetwork);

			let mut withdraw = Withdraw {
				id: Self::new_random_id(),
				asset_id,
				amount,
				destination: beneficiary.clone(),
				is_blocked: false,
				extra: Vec::new(),
			};
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(network);
			let metadata =
				<Metadata<T>>::get(withdraw.asset_id).ok_or(Error::<T>::AssetNotRegistered)?;

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

			// Pay the fees
			<T as Config>::Currency::transfer(
				&user,
				&Self::thea_account(),
				total_fees.saturated_into(),
				Preservation::Preserve,
			)?;

			// Withdraw assets
			Self::resolver_withdraw(asset_id.into(), amount, &user, Self::thea_account())?;

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

		pub fn do_deposit(network: Network, payload: Vec<u8>) -> Result<(), DispatchError> {
			match network {
				PARACHAIN_NETWORK => Self::parachain_deposit(network, payload)?,
				ETHEREUM_NETWORK => Self::ethereum_deposit(network, payload)?,
				x => {
					log::error!(target:"engine","Unknown Thea network id in deposit: {:?}",x)
				}
			}
			Ok(())
		}

		pub fn ethereum_deposit(network: Network, payload: Vec<u8>) -> Result<(), DispatchError> {
			// 1. Decode the payload
			let message: EthereumOP<T::AccountId> =
				Decode::decode(&mut &payload[..]).map_err(|_| Error::<T>::FailedToDecode)?;
			// 2. Execute the payload
			// TODO: Add logic to take txn fees in incoming deposits if PDEX is not available
			match message.action {
				EtherumAction::Deposit(asset_id, amount, user) => {
					Self::regular_ethereum_deposit(
						network,
						message.txn_id,
						asset_id,
						amount,
						user,
					)?;
				},
				EtherumAction::DepositToOrderbook(asset_id, amount, main, proxy) => {
					let deposit = Self::regular_ethereum_deposit(
						network,
						message.txn_id,
						asset_id,
						amount,
						main.clone(),
					)?;
					// Check if main is registered user in OCEX, if not register it
					if !pallet_ocex_lmp::Pallet::<T>::check_main_account_registration(&main) {
						pallet_ocex_lmp::Pallet::<T>::register_user(main.clone(), proxy)?;
					} else {
						// Check if proxy is registered under main in OCEX, if not register it
						let (flag, num) =
							pallet_ocex_lmp::Pallet::<T>::check_if_proxy_is_registered(
								&main, &proxy,
							);
						if !flag && num < polkadex_primitives::ProxyLimit::get().saturated_into() {
							pallet_ocex_lmp::Pallet::<T>::add_proxy(main.clone(), proxy)?;
						} else {
							// Drop the proxy silently
						}
					}
					// Call deposit for user in OCEX.
					pallet_ocex_lmp::Pallet::<T>::do_deposit(
						main,
						polkadex_primitives::AssetId::Asset(deposit.asset_id),
						deposit.amount.saturated_into(),
					)?;
				},
				EtherumAction::Swap => {
					todo!()
				},
			}
			Ok(())
		}

		pub fn regular_ethereum_deposit(
			network: Network,
			txn_id: H256,
			asset_id: u128,
			amount: u128, // Already in 10^12
			recipient: T::AccountId,
		) -> Result<Deposit<T::AccountId>, DispatchError> {
			let deposit: Deposit<T::AccountId> = Deposit{id: txn_id.encode(), asset_id, recipient, amount, extra: Vec::new()};
			Self::execute_deposit(network, deposit.clone())?;
			Ok(deposit)
		}

		pub fn parachain_deposit(network: Network, payload: Vec<u8>) -> Result<(), DispatchError> {
			let mut deposit: Deposit<T::AccountId> =
				Decode::decode(&mut &payload[..]).map_err(|_| Error::<T>::FailedToDecode)?;

			// Get the metadata
			let metadata =
				<Metadata<T>>::get(deposit.asset_id).ok_or(Error::<T>::AssetNotRegistered)?;

			deposit.amount = deposit.amount_in_native_decimals(metadata); // Convert amount to native form

			Self::execute_deposit(network, deposit)?;
			Ok(())
		}

		pub fn execute_deposit(
			network: Network,
			deposit: Deposit<T::AccountId>,
		) -> Result<(), DispatchError> {
			Self::resolver_deposit(
				deposit.asset_id.into(),
				// Convert the decimals config
				deposit.amount,
				&deposit.recipient,
				Self::thea_account(),
				1u128,
				Self::thea_account(),
			)?;

			// TODO: It's here to not break the indexing workflow, remove it later.
			Self::deposit_event(Event::<T>::DepositApproved(
				network,
				deposit.recipient.clone(),
				deposit.asset_id,
				deposit.amount,
				deposit.id.clone(),
			));

			// Emit event
			Self::deposit_event(Event::<T>::DepositClaimed(
				deposit.recipient.clone(),
				deposit.asset_id,
				deposit.amount,
				deposit.id,
			));
			Ok(())
		}
	}

	impl<T: Config> TheaIncomingExecutor for Pallet<T> {
		fn execute_deposits(network: Network, deposits: Vec<u8>) {
			if let Err(error) = Self::do_deposit(network, deposits) {
				log::error!(target:"thea","Deposit Failed : {:?}", error);
			}
		}
	}

	// Implement this trait for handing deposits and withdrawals
	impl<T: Config>
		polkadex_primitives::assets::Resolver<
			T::AccountId,
			T::Currency,
			T::Assets,
			T::AssetId,
			T::NativeAssetId,
		> for Pallet<T>
	{
	}
}
