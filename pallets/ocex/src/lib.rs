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

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::Get,
	traits::{fungibles::Mutate, Currency, ExistenceRequirement},
};
use frame_system::ensure_signed;

use polkadex_primitives::assets::AssetId;

use pallet_timestamp::{self as timestamp};
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
use sp_std::prelude::*;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[cfg(test)]
mod tests;

mod benchmarking;
mod types;
pub mod weights;

pub use weights::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use super::*;
	use crate::types::TradingPairStatus;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles::{Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ias_verify::{verify_ias_report, SgxStatus};
	use polkadex_primitives::{
		assets::AssetId,
		ocex::{AccountInfo, EnclaveSnapshot, TradingPairInfo, Withdrawal},
		ProxyLimit, Signature, SnapshotAccLimit, WithdrawalLimit,
	};
	use sp_core::{crypto::AccountId32, H256};
	use sp_io::hashing::blake2_256;
	use sp_runtime::traits::{IdentifyAccount, Member, Verify};
	use sp_std::vec::Vec;

	/// Our pallet's configuration trait. All our types and constants go in here. If the
	/// pallet is dependent on specific other pallets, then their configuration traits
	/// should be added to our implied traits list.
	///
	/// `frame_system::Config` should always be included.
	#[pallet::config]
	pub trait Config: frame_system::Config + timestamp::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Address which holds the customer funds.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Balances Pallet
		type NativeCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Assets Pallet
		type OtherAssets: Mutate<
				<Self as frame_system::Config>::AccountId,
				Balance = BalanceOf<Self>,
				AssetId = u128,
			> + Inspect<<Self as frame_system::Config>::AccountId>;

		/// Origin that can send orderbook snapshots and withdrawal requests
		type EnclaveOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
		type Public: Clone
			+ PartialEq
			+ IdentifyAccount<AccountId = Self::AccountId>
			+ core::fmt::Debug
			+ codec::Codec
			+ Ord
			+ scale_info::TypeInfo;

		/// A matching `Signature` type.
		type Signature: Verify<Signer = Self::Public>
			+ Clone
			+ PartialEq
			+ core::fmt::Debug
			+ codec::Codec
			+ scale_info::TypeInfo;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		// declared number of milliseconds per day and is used to determine
		// enclave's report validity time.
		// standard 24h in ms = 86_400_000
		type MsPerDay: Get<Self::Moment>;
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::error]
	pub enum Error<T> {
		RegisterationShouldBeSignedByMainAccount,
		TradingPairIsNotOperational,
		MainAccountAlreadyRegistered,
		SnapshotNonceError,
		EnclaveSignatureVerificationFailed,
		MainAccountNotFound,
		ProxyLimitExceeded,
		TradingPairAlreadyRegistered,
		BothAssetsCannotBeSame,
		TradingPairNotFound,
		/// Provided Report Value is invalid
		InvalidReportValue,
		/// IAS attestation verification failed:
		/// a) certificate[s] outdated;
		/// b) enclave is not properly signed it's report with IAS service;
		RemoteAttestationVerificationFailed,
		/// Sender has not been attested
		SenderIsNotAttestedEnclave,
		/// RA status is insufficient
		InvalidSgxReportStatus,
		/// Storage overflow ocurred
		StorageOverflow,
		/// ProxyNotFound
		ProxyNotFound
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// What to do at the end of each block.
		///
		/// Clean IngressMessages
		fn on_initialize(n: T::BlockNumber) -> Weight {
			// When block's been initialized - clean up expired registrations of enclaves
			Self::unregister_timed_out_enclaves();
			<IngressMessages<T>>::put(Vec::<
				polkadex_primitives::ocex::IngressMessages<T::AccountId, BalanceOf<T>>,
			>::new());
			// TODO: Benchmark on initialize
			0
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers a new account in orderbook
		#[pallet::weight(10000)]
		pub fn register_main_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(
				!<Accounts<T>>::contains_key(&main_account),
				Error::<T>::MainAccountAlreadyRegistered
			);
			<Accounts<T>>::insert(&main_account, AccountInfo::new(proxy.clone()));
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ocex::IngressMessages::RegisterUser(
					main_account.clone(),
					proxy.clone(),
				));
			});
			Self::deposit_event(Event::MainAccountRegistered { main: main_account, proxy });
			Ok(())
		}

		/// Adds a proxy account to a pre-registered main acocunt
		#[pallet::weight(10000)]
		pub fn add_proxy_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::contains_key(&main_account), Error::<T>::MainAccountNotFound);
			if let Some(mut account_info) = <Accounts<T>>::get(&main_account) {
				ensure!(
					account_info.add_proxy(proxy.clone()).is_ok(),
					Error::<T>::ProxyLimitExceeded
				);

				<IngressMessages<T>>::mutate(|ingress_messages| {
					ingress_messages.push(polkadex_primitives::ocex::IngressMessages::AddProxy(
						main_account.clone(),
						proxy.clone(),
					));
				});
				<Accounts<T>>::insert(&main_account, account_info);
				Self::deposit_event(Event::MainAccountRegistered { main: main_account, proxy });
			}
			Ok(())
		}

		/// Removes a proxy account from pre-registered main acocunt
		#[pallet::weight(10000)]
		pub fn remove_proxy_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::contains_key(&main_account), Error::<T>::MainAccountNotFound);
			<Accounts<T>>::try_mutate(&main_account, |account_info| {
				if let Some(account_info) = account_info {
					let proxy_positon = account_info.proxies.iter().position(|account| *account == proxy).ok_or(Error::<T>::ProxyNotFound)?;
					account_info.proxies.remove(proxy_positon);
					<IngressMessages<T>>::mutate(|ingress_messages| {
						ingress_messages.push(polkadex_primitives::ocex::IngressMessages::RemoveProxy(
							main_account.clone(),
							proxy.clone(),
						));
					});
				}
				Ok(())
			})
		}

		/// Registers a new trading pair
		#[pallet::weight(10000)]
		pub fn register_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
			minimum_trade_amount: BalanceOf<T>,
			maximum_trade_amount: BalanceOf<T>,
			minimum_qty_amount: BalanceOf<T>,
			minimum_withdrawal_amount: BalanceOf<T>,
			minimum_deposit_amount: BalanceOf<T>,
			maximum_withdrawal_amount: BalanceOf<T>,
			maximum_deposit_amount: BalanceOf<T>,
			base_withdrawal_fee: BalanceOf<T>,
			quote_withdrawal_fee: BalanceOf<T>,
			enclave_id: T::AccountId,
			min_depth: BalanceOf<T>,
			max_spread: BalanceOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				!<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairAlreadyRegistered
			);
			ensure!(
				!<TradingPairs<T>>::contains_key(&quote, &base),
				Error::<T>::TradingPairAlreadyRegistered
			);

			let trading_pair_info = TradingPairInfo::new(
				base,
				quote,
				minimum_trade_amount,
				maximum_trade_amount,
				minimum_qty_amount,
				minimum_withdrawal_amount,
				minimum_deposit_amount,
				maximum_withdrawal_amount,
				maximum_deposit_amount,
				base_withdrawal_fee,
				quote_withdrawal_fee,
				enclave_id,
				min_depth,
				max_spread,
			);
			<TradingPairs<T>>::insert(&base, &quote, trading_pair_info.clone());
			<TradingPairsStatus<T>>::insert(&base, &quote, TradingPairStatus::new());
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ocex::IngressMessages::StartEnclave(
					trading_pair_info,
				));
			});
			Self::deposit_event(Event::TradingPairRegistered { base, quote });
			Ok(())
		}

		/// Deposit Assets to Orderbook
		#[pallet::weight(10000)]
		pub fn deposit(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
			amount: BalanceOf<T>,
			is_base: bool,
		) -> DispatchResult {
			let main = ensure_signed(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairNotFound
			);
			let trading_pair_status = <TradingPairsStatus<T>>::get(&base, &quote)
				.ok_or(Error::<T>::TradingPairNotFound)?;
			ensure!(trading_pair_status.is_active, Error::<T>::TradingPairIsNotOperational);

			let asset = if is_base { base } else { quote };
			Self::transfer_asset(&main, &Self::get_custodian_account(), amount, asset)?;
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages
					.push(polkadex_primitives::ocex::IngressMessages::Deposit(main, asset, amount));
			});
			Self::deposit_event(Event::DepositSuccessful { pair: (base, quote), asset, amount });
			Ok(())
		}

		/// Extrinsic used by enclave to submit balance snapshot and withdrawal requests
		#[pallet::weight(10000)]
		pub fn submit_snapshot(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
			mut snapshot: EnclaveSnapshot<
				T::AccountId,
				BalanceOf<T>,
				SnapshotAccLimit,
				WithdrawalLimit,
			>,
			signature: T::Signature,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairNotFound
			);
			let last_snapshot_serial_number =
				if let Some(last_snapshot) = <Snapshots<T>>::get(&base, &quote) {
					last_snapshot.snapshot_number
				} else {
					0
				};
			ensure!(
				snapshot.snapshot_number.eq(&last_snapshot_serial_number.saturating_add(1)),
				Error::<T>::SnapshotNonceError
			);
			let trading_pair_info =
				<TradingPairs<T>>::get(&base, &quote).ok_or(Error::<T>::TradingPairNotFound)?;

			let bytes = snapshot.encode();

			ensure!(
				signature.verify(bytes.as_slice(), &(trading_pair_info.enclave_id)),
				Error::<T>::EnclaveSignatureVerificationFailed
			);
			<Withdrawals<T>>::insert((base, quote), snapshot.snapshot_number, snapshot.withdrawals);
			snapshot.withdrawals =
				BoundedVec::<Withdrawal<T::AccountId, BalanceOf<T>>, WithdrawalLimit>::default();
			<Snapshots<T>>::insert(&base, &quote, snapshot);
			Ok(())
		}

		/// Extrinsic used to emit a shutdown request of an Enclave
		#[pallet::weight(10000)]
		pub fn shutdown_enclave(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairNotFound
			);
			ensure!(
				<TradingPairs<T>>::contains_key(&quote, &base),
				Error::<T>::TradingPairNotFound
			);

			let trading_pair_info =
				<TradingPairs<T>>::get(&base, &quote).ok_or(Error::<T>::TradingPairNotFound)?;

			<TradingPairsStatus<T>>::mutate(&quote, &base, |status_option| {
				if let Some(status) = status_option {
					status.is_active = false;
					<IngressMessages<T>>::mutate(|ingress_messages| {
						ingress_messages.push(
							polkadex_primitives::ocex::IngressMessages::ShutdownEnclave(
								trading_pair_info.enclave_id.clone(),
							),
						)
					});
					Self::deposit_event(Event::EnclaveShutdownRequest {
						id: trading_pair_info.enclave_id,
					});
					Some(status.clone())
				} else {
					None
				}
			});
			Ok(())
		}

		/// In order to register itself - enclave must send it's own report to this extrinsic
		#[pallet::weight(0 + T::DbWeight::get().writes(1))]
		pub fn register_enclave(origin: OriginFor<T>, ias_report: Vec<u8>) -> DispatchResult {
			let _relayer = ensure_signed(origin)?;

			use sp_runtime::SaturatedConversion;

			let report = verify_ias_report(&ias_report)
				.map_err(|_| <Error<T>>::RemoteAttestationVerificationFailed)?;

			// TODO: attested key verification enabled
			let enclave_signer = T::AccountId::decode(&mut &report.pubkey[..])
				.map_err(|_| <Error<T>>::SenderIsNotAttestedEnclave)?;

			// TODO: any other checks we want to run?
			ensure!(
				(report.status == SgxStatus::Ok) |
					(report.status == SgxStatus::ConfigurationNeeded),
				<Error<T>>::InvalidSgxReportStatus
			);
			let new_enclave = (enclave_signer.clone(), T::Moment::saturated_from(report.timestamp));
			<RegisteredEnclaves<T>>::mutate(|v| {
				if let Some(v) = v {
					if let Some(existing) = v.iter().position(|(e, _)| e == &enclave_signer) {
						v[existing] = new_enclave;
					} else {
						v.push(new_enclave);
					}
					Some(v.clone())
				} else {
					Some(sp_std::vec![new_enclave])
				}
			});
			Self::deposit_event(Event::EnclaveRegistered(enclave_signer));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// clean-up function - should be called on each block
		fn unregister_timed_out_enclaves() {
			use sp_runtime::traits::CheckedSub;
			<RegisteredEnclaves<T>>::mutate(|v| {
				if let Some(v) = v {
					v.retain(|(_, attested_ts)| {
						<timestamp::Pallet<T>>::get().checked_sub(&attested_ts).unwrap() <
							T::MsPerDay::get()
					});
				}
			});
			Self::deposit_event(Event::EnclaveCleanup);
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MainAccountRegistered { main: T::AccountId, proxy: T::AccountId },
		TradingPairRegistered { base: AssetId, quote: AssetId },
		DepositSuccessful { pair: (AssetId, AssetId), asset: AssetId, amount: BalanceOf<T> },
		EnclaveShutdownRequest { id: T::AccountId },
		EnclaveRegistered(T::AccountId),
		EnclaveCleanup,
		TradingPairIsNotOperational,
	}

	// A map that has enumerable entries.
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub(super) type Accounts<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		AccountInfo<T::AccountId, BalanceOf<T>, ProxyLimit>,
		OptionQuery,
	>;

	// Trading pairs registered as Base, Quote => TradingPairInfo
	#[pallet::storage]
	#[pallet::getter(fn trading_pairs)]
	pub(super) type TradingPairs<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetId,
		Blake2_128Concat,
		AssetId,
		TradingPairInfo<T::AccountId, BalanceOf<T>>,
		OptionQuery,
	>;

	// Operational Status of registered trading pairs
	#[pallet::storage]
	#[pallet::getter(fn trading_pairs_status)]
	pub(super) type TradingPairsStatus<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetId,
		Blake2_128Concat,
		AssetId,
		TradingPairStatus,
		OptionQuery,
	>;

	// Snapshots of all trading pairs
	#[pallet::storage]
	#[pallet::getter(fn snapshots)]
	pub(super) type Snapshots<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetId,
		Blake2_128Concat,
		AssetId,
		EnclaveSnapshot<T::AccountId, BalanceOf<T>, SnapshotAccLimit, WithdrawalLimit>,
		OptionQuery,
	>;

	// Withdrawals mapped by their trading pairs and snapshot numbers
	#[pallet::storage]
	#[pallet::getter(fn withdrawals)]
	pub(super) type Withdrawals<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		(AssetId, AssetId),
		Blake2_128Concat,
		u32,
		BoundedVec<Withdrawal<T::AccountId, BalanceOf<T>>, WithdrawalLimit>,
		ValueQuery,
	>;

	// Queue for enclave ingress messages
	#[pallet::storage]
	#[pallet::getter(fn ingress_messages)]
	pub(super) type IngressMessages<T: Config> = StorageValue<
		_,
		Vec<polkadex_primitives::ocex::IngressMessages<T::AccountId, BalanceOf<T>>>,
		ValueQuery,
	>;

	// Vector of registered enclaves
	#[pallet::storage]
	#[pallet::getter(fn get_registered_enclaves)]
	pub(super) type RegisteredEnclaves<T> = StorageValue<
		_,
		Vec<(<T as frame_system::Config>::AccountId, <T as timestamp::Config>::Moment)>,
	>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	/// Returns the AccountId to hold user funds, note this account has no private keys and
	/// can accessed using on-chain logic.
	fn get_custodian_account() -> T::AccountId {
		T::PalletId::get().into_account()
	}

	fn transfer_asset(
		payer: &T::AccountId,
		payee: &T::AccountId,
		amount: BalanceOf<T>,
		asset: AssetId,
	) -> DispatchResult {
		match asset {
			AssetId::polkadex => {
				T::NativeCurrency::transfer(
					payer,
					payee,
					amount.unique_saturated_into(),
					ExistenceRequirement::KeepAlive,
				)?;
			},
			AssetId::asset(id) => {
				T::OtherAssets::teleport(id, payer, payee, amount.unique_saturated_into())?;
			},
		}
		Ok(())
	}

	fn _submit_state() -> Result<(), Error<T>> {
		todo!()
	}
}
