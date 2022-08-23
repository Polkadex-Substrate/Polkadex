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
mod mock;

#[cfg(test)]
mod tests;

mod benchmarking;
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
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles::{Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_support::storage::bounded_btree_map::BoundedBTreeMap;
	use frame_system::pallet_prelude::*;
	use ias_verify::{verify_ias_report, SgxStatus};
	use polkadex_primitives::{assets::AssetId, ocex::{AccountInfo, TradingPairConfig}, snapshot::EnclaveSnapshot, withdrawal::Withdrawal, ProxyLimit, WithdrawalLimit, AssetsLimit, AccountId, SnapshotAccLimit};
	use sp_runtime::SaturatedConversion;
	use polkadex_primitives::snapshot::Fees;
	use sp_runtime::traits::{IdentifyAccount, Verify};
	use sp_std::vec::Vec;
	// use polkadex_primitives::SnapshotAccLimit;

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

		/// Governance Origin
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
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
		/// Caller is not authorized to claim the withdrawal.
		/// Normally, when Sender != main_account.
		SenderNotAuthorizedToWithdraw,
		InvalidWithdrawalIndex,
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// What to do at the end of each block.
		///
		/// Clean IngressMessages
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			// When block's been initialized - clean up expired registrations of enclaves
			//Self::unregister_timed_out_enclaves(); FIXME: Commented out for testing. Should be restored before mainnet launch
			if let Some(snapshot_nonce) = <SnapshotNonce<T>>::get() {
				if let Some(snapshot) = <Snapshots<T>>::get(snapshot_nonce.saturating_sub(1)) {
					<IngressMessages<T>>::put(Vec::<
						polkadex_primitives::ingress::IngressMessages<T::AccountId, BalanceOf<T>>,
					>::from([polkadex_primitives::ingress::IngressMessages::LastestSnapshot(snapshot.merkle_root, snapshot.snapshot_number)]));
				} else {
					<IngressMessages<T>>::put(Vec::<
						polkadex_primitives::ingress::IngressMessages<T::AccountId, BalanceOf<T>>,
					>::new());
				}
			} else {
				<IngressMessages<T>>::put(Vec::<
					polkadex_primitives::ingress::IngressMessages<T::AccountId, BalanceOf<T>>,
				>::new());
			}
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

			let mut account_info = AccountInfo::new(main_account.clone());
			ensure!(account_info.add_proxy(proxy.clone()).is_ok(), Error::<T>::ProxyLimitExceeded);
			<Accounts<T>>::insert(&main_account, account_info);

			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ingress::IngressMessages::RegisterUser(
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
					ingress_messages.push(polkadex_primitives::ingress::IngressMessages::AddProxy(
						main_account.clone(),
						proxy.clone(),
					));
				});
				<Accounts<T>>::insert(&main_account, account_info);
				Self::deposit_event(Event::MainAccountRegistered { main: main_account, proxy });
			}
			Ok(())
		}

		/// Registers a new trading pair
		#[pallet::weight(10000)]
		pub fn close_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairNotFound
			);

			if let Some(trading_pair) = <TradingPairs<T>>::get(&base, &quote) {
				<TradingPairsStatus<T>>::mutate(&base, &quote, |status| *status = false);
				<IngressMessages<T>>::mutate(|ingress_messages| {
					ingress_messages.push(
						polkadex_primitives::ingress::IngressMessages::CloseTradingPair(
							trading_pair.clone(),
						),
					);
				});
				Self::deposit_event(Event::ShutdownTradingPair { pair: trading_pair });
			}
			Ok(())
		}

		/// Registers a new trading pair
		#[pallet::weight(10000)]
		pub fn open_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairNotFound
			);

			if let Some(trading_pair) = <TradingPairs<T>>::get(&base, &quote) {
				<TradingPairsStatus<T>>::mutate(&base, &quote, |status| *status = true);
				<IngressMessages<T>>::mutate(|ingress_messages| {
					ingress_messages.push(
						polkadex_primitives::ingress::IngressMessages::OpenTradingPair(
							trading_pair.clone(),
						),
					);
				});
				Self::deposit_event(Event::OpenTradingPair { pair: trading_pair });
			}
			Ok(())
		}

		/// Registers a new trading pair
		#[pallet::weight(10000)]
		pub fn register_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
			min_trade_amount: BalanceOf<T>,
			max_trade_amount: BalanceOf<T>,
			min_order_qty: BalanceOf<T>,
			max_order_qty: BalanceOf<T>,
			max_spread: BalanceOf<T>,
			min_depth: BalanceOf<T>
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				!<TradingPairs<T>>::contains_key(&base, &quote),
				Error::<T>::TradingPairAlreadyRegistered
			);
			ensure!(
				!<TradingPairs<T>>::contains_key(&quote, &base),
				Error::<T>::TradingPairAlreadyRegistered
			);

			// TODO: Check if base and quote assets are enabled for deposits
			let trading_pair_info = TradingPairConfig {
				base_asset: base,
				quote_asset: quote,
				min_trade_amount,
				max_trade_amount,
				min_order_qty,
				max_order_qty,
				max_spread,
				min_depth
			};
			<TradingPairs<T>>::insert(&base, &quote, trading_pair_info.clone());
			<TradingPairsStatus<T>>::insert(&base, &quote, true);
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(
					polkadex_primitives::ingress::IngressMessages::OpenTradingPair(
						trading_pair_info,
					),
				);
			});
			Self::deposit_event(Event::TradingPairRegistered { base, quote });
			Ok(())
		}

		/// Deposit Assets to Orderbook
		#[pallet::weight(10000)]
		pub fn deposit(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			// TODO: Check if asset is enabled for deposit
			Self::transfer_asset(&user, &Self::get_custodian_account(), amount, asset)?;
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ingress::IngressMessages::Deposit(
					user.clone(),
					asset,
					amount,
				));
			});
			Self::deposit_event(Event::DepositSuccessful { user, asset, amount });
			Ok(())
		}

		/// Extrinsic used by enclave to submit balance snapshot and withdrawal requests
		#[pallet::weight(10000)]
		pub fn submit_snapshot(
			origin: OriginFor<T>,
			mut snapshot: EnclaveSnapshot<T::AccountId, BalanceOf<T>, WithdrawalLimit,AssetsLimit, SnapshotAccLimit>,
			signature: T::Signature,
		) -> DispatchResult {
			let enclave = ensure_signed(origin)?;
			ensure!(
				<RegisteredEnclaves<T>>::contains_key(&enclave),
				Error::<T>::SenderIsNotAttestedEnclave
			);

			let last_snapshot_serial_number =
				if let Some(last_snapshot_number) = <SnapshotNonce<T>>::get() {
					last_snapshot_number
				} else {
					0
				};
			ensure!(
				snapshot.snapshot_number.eq(&last_snapshot_serial_number),
				Error::<T>::SnapshotNonceError
			);
			let bytes = snapshot.encode();
			ensure!(
				signature.verify(bytes.as_slice(), &enclave),
				Error::<T>::EnclaveSignatureVerificationFailed
			);
			<Withdrawals<T>>::insert(snapshot.snapshot_number, snapshot.withdrawals);
			<FeesCollected<T>>::insert(snapshot.snapshot_number,snapshot.fees.clone());
			snapshot.withdrawals = Default::default();
			<Snapshots<T>>::insert(snapshot.snapshot_number, snapshot);
			<SnapshotNonce<T>>::put(last_snapshot_serial_number.saturating_add(1));
			Ok(())
		}

		// FIXME Only for testing will be removed before mainnet launch
		/// Insert Enclave
		#[doc(hidden)]
		#[pallet::weight(10000 + T::DbWeight::get().writes(1))]
		pub fn insert_enclave(
			origin: OriginFor<T>,
		    encalve: T::AccountId
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<RegisteredEnclaves<T>>::insert(encalve, T::Moment::from(T::MsPerDay::get() * T::Moment::from(10000u32)));
			Ok(())
		}

		/// Withdraws Fees Collected
		///
		/// params:  snapshot_number: u32
		#[pallet::weight(10000 + T::DbWeight::get().writes(1))]
		pub fn collect_fees(
			origin: OriginFor<T>,
			snapshot_id: u32,
			beneficiary: T::AccountId
		) -> DispatchResult {
			// TODO: The caller should be of operational council
			let _sender = ensure_signed(origin)?;

			let fees: Vec<Fees<BalanceOf<T>>> = <FeesCollected<T>>::get(snapshot_id).iter().cloned().collect();
			for fee in fees {
				Self::transfer_asset(
					&Self::get_custodian_account(),
					&beneficiary,
					fee.amount,
					fee.asset,
				)?;
			}
			Self::deposit_event(Event::FeesClaims {
				beneficiary: beneficiary,
				snapshot_id
			});
			Ok(())
		}

		/// Extrinsic used to shutdown the orderbook
		#[pallet::weight(10000)]
		pub fn shutdown(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<ExchangeState<T>>::put(false);
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ingress::IngressMessages::Shutdown);
			});
			Ok(())
		}

		/// Withdraws user balance
		///
		/// params: pair: (base,quote), snapshot_number: u32
		#[pallet::weight(10000 + T::DbWeight::get().writes(1))]
		pub fn withdraw(
			origin: OriginFor<T>,
			snapshot_id: u32,

		) -> DispatchResult {
			// Anyone can claim the withdrawal for any user
			// This is to build services that can enable free withdrawals similar to CEXes.
			let sender = ensure_signed(origin)?;

			let mut withdrawals: BoundedBTreeMap<T::AccountId, BoundedVec<Withdrawal<T::AccountId, BalanceOf<T>>, WithdrawalLimit>, SnapshotAccLimit> = <Withdrawals<T>>::get(snapshot_id);
			ensure!(
				withdrawals.contains_key(&sender),
				Error::<T>::InvalidWithdrawalIndex
			); 
			if let Some(withdrawal_vector) = withdrawals.get(&sender){
				for x in withdrawal_vector.iter(){
					Self::transfer_asset(
						&Self::get_custodian_account(),
						&x.main_account,
						x.amount,
						x.asset,
					)?;
				}
				Self::deposit_event(Event::WithdrawalClaimed {
					main: sender.clone(),
					withdrawals: withdrawal_vector.to_owned()
				});
			}
			/* withdrawals.remove(&sender);
			<Withdrawals<T>>::insert(snapshot_id, withdrawals); */
			Ok(())
		}

		/// In order to register itself - enclave must send it's own report to this extrinsic
		#[pallet::weight(0 + T::DbWeight::get().writes(1))]
		pub fn register_enclave(origin: OriginFor<T>, ias_report: Vec<u8>) -> DispatchResult {
			let relayer = ensure_signed(origin)?;
			if cfg!(not(debug_assertions)) {
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
				<RegisteredEnclaves<T>>::mutate(&enclave_signer, |v| {
					*v = Some(T::Moment::saturated_from(report.timestamp));
				});
				Self::deposit_event(Event::EnclaveRegistered(enclave_signer));
			} else {
				<RegisteredEnclaves<T>>::mutate(&relayer, |v| {
					*v = Some(T::Moment::default());
				});
				Self::deposit_event(Event::EnclaveRegistered(relayer));
			}
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// clean-up function - should be called on each block
		fn unregister_timed_out_enclaves() {
			use sp_runtime::traits::CheckedSub;
			let mut enclave_to_remove = sp_std::vec![];
			let iter = <RegisteredEnclaves<T>>::iter();
			iter.for_each(|(enclave, attested_ts)| {
				if <timestamp::Pallet<T>>::get().checked_sub(&attested_ts).unwrap() >=
					T::MsPerDay::get()
				{
					enclave_to_remove.push(enclave);
				}
			});
			for enclave in &enclave_to_remove {
				<RegisteredEnclaves<T>>::remove(enclave);
			}
			Self::deposit_event(Event::EnclaveCleanup(enclave_to_remove));
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FeesClaims{
			beneficiary: T::AccountId,
			snapshot_id: u32
		},
		MainAccountRegistered {
			main: T::AccountId,
			proxy: T::AccountId,
		},
		TradingPairRegistered {
			base: AssetId,
			quote: AssetId,
		},
		DepositSuccessful {
			user: T::AccountId,
			asset: AssetId,
			amount: BalanceOf<T>,
		},
		ShutdownTradingPair {
			pair: TradingPairConfig<BalanceOf<T>>,
		},
		OpenTradingPair {
			pair: TradingPairConfig<BalanceOf<T>>,
		},
		EnclaveRegistered(T::AccountId),
		EnclaveCleanup(Vec<T::AccountId>),
		TradingPairIsNotOperational,
		WithdrawalClaimed {
			main: T::AccountId,
			withdrawals: BoundedVec<Withdrawal<T::AccountId, BalanceOf<T>>, WithdrawalLimit>
		},
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
		TradingPairConfig<BalanceOf<T>>,
		OptionQuery,
	>;

	// Operational Status of registered trading pairs
	#[pallet::storage]
	#[pallet::getter(fn trading_pairs_status)]
	pub(super) type TradingPairsStatus<T: Config> =
	StorageDoubleMap<_, Blake2_128Concat, AssetId, Blake2_128Concat, AssetId, bool, ValueQuery>;

	// Snapshots Storage
	#[pallet::storage]
	#[pallet::getter(fn snapshots)]
	pub(super) type Snapshots<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		EnclaveSnapshot<T::AccountId, BalanceOf<T>, WithdrawalLimit,AssetsLimit, SnapshotAccLimit>,
		OptionQuery,
	>;

	// Snapshots Nonce
	#[pallet::storage]
	#[pallet::getter(fn snapshot_nonce)]
	pub(super) type SnapshotNonce<T: Config> = StorageValue<_, u32, OptionQuery>;

	// Exchange Operation State
	#[pallet::storage]
	#[pallet::getter(fn orderbook_operational_state)]
	pub(super) type ExchangeState<T: Config> = StorageValue<_, bool, ValueQuery>;


	// Fees collected
	#[pallet::storage]
	#[pallet::getter(fn fees_collected)]
	pub(super) type FeesCollected<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BoundedVec<Fees<BalanceOf<T>>, AssetsLimit>,
		ValueQuery,
	>;

	// Withdrawals mapped by their trading pairs and snapshot numbers
	#[pallet::storage]
	#[pallet::getter(fn withdrawals)]
	pub(super) type Withdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		u32,
		BoundedBTreeMap<T::AccountId,BoundedVec<Withdrawal<T::AccountId, BalanceOf<T>>, WithdrawalLimit>,SnapshotAccLimit>,
		ValueQuery,
	>;

	// Queue for enclave ingress messages
	#[pallet::storage]
	#[pallet::getter(fn ingress_messages)]
	pub(super) type IngressMessages<T: Config> = StorageValue<
		_,
		Vec<polkadex_primitives::ingress::IngressMessages<T::AccountId, BalanceOf<T>>>,
		ValueQuery,
	>;

	// Vector of registered enclaves
	#[pallet::storage]
	#[pallet::getter(fn get_registered_enclaves)]
	pub(super) type RegisteredEnclaves<T: Config> =
	StorageMap<_, Blake2_128Concat, T::AccountId, T::Moment, OptionQuery>;
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
		T::PalletId::get().into_account_truncating()
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
}
