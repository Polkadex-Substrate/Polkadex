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

use codec::Encode;
use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::Get,
	traits::{fungibles::Mutate, Currency, ExistenceRequirement, OneSessionHandler},
};

use frame_system::ensure_signed;
use polkadex_primitives::{assets::AssetId, DigestItem, OnChainEventsLimit};

use ocex_primitives::{AuthorityId, AuthorityIndex, ConsensusLog, OCEX_ENGINE_ID};
use pallet_timestamp::{self as timestamp};
use sp_runtime::traits::{AccountIdConversion, IsMember, UniqueSaturatedInto};
use sp_std::prelude::*;

// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod unsigned;
pub mod weights;

pub use weights::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const DEPOSIT_MAX: u128 = 1_000_000_000_000_000_000_000_000_000;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
	// Import various types used to declare pallet in scope.
	use super::*;
	use core::ops::Div;
	use frame_support::{
		pallet_prelude::*,
		storage::bounded_btree_map::BoundedBTreeMap,
		traits::{
			fungibles::{Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::{offchain::CreateSignedTransaction, pallet_prelude::*};
	use polkadex_primitives::{
		assets::AssetId,
		ocex::{AccountInfo, TradingPairConfig},
		snapshot::{EnclaveSnapshot, Fees},
		withdrawal::Withdrawal,
		AssetsLimit, ProxyLimit, SnapshotAccLimit, WithdrawalLimit, UNIT_BALANCE,
	};
	use rust_decimal::{prelude::ToPrimitive, Decimal};
	use sp_application_crypto::RuntimeAppPublic;
	use sp_runtime::{
		traits::{IdentifyAccount, Saturating, Verify},
		SaturatedConversion,
	};
	use sp_std::vec::Vec;

	type WithdrawalsMap<T> = BoundedBTreeMap<
		<T as frame_system::Config>::AccountId,
		BoundedVec<Withdrawal<<T as frame_system::Config>::AccountId>, WithdrawalLimit>,
		SnapshotAccLimit,
	>;

	type EnclaveSnapshotType<T> = EnclaveSnapshot<
		<T as frame_system::Config>::AccountId,
		WithdrawalLimit,
		AssetsLimit,
		SnapshotAccLimit,
	>;

	/// Our pallet's configuration trait. All our types and constants go in here. If the
	/// pallet is dependent on specific other pallets, then their configuration traits
	/// should be added to our implied traits list.
	///
	/// `frame_system::Config` should always be included.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + timestamp::Config + CreateSignedTransaction<Call<Self>>
	{
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// Address which holds the customer funds.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// AuthorityID
		type OCEXId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ MaybeSerializeDeserialize
			+ Ord
			+ MaxEncodedLen;

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
		type Signature: Verify<Signer = <Self as pallet::Config>::Public>
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
		/// Unable to convert given balance to internal Decimal data type
		FailedToConvertDecimaltoBalance,
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
		AmountOverflow,
		///ProxyNotFound
		ProxyNotFound,
		/// MinimumOneProxyRequried
		MinimumOneProxyRequired,
		/// Onchain Events vector is full
		OnchainEventsBoundedVecOverflow,
		/// Overflow of Deposit amount
		DepositOverflow,
		/// Enclave not whitelisted
		EnclaveNotWhitelisted,
		/// Trading Pair is not registed for updating
		TradingPairNotRegistered,
		/// Trading Pair config value cannot be set to zero
		TradingPairConfigCannotBeZero,
		/// Client submitted report signature invalid
		ReportSignatureInvalid,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// What to do at the end of each block.
		///
		/// Clean IngressMessages
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			// When block's been initialized - clean up expired registrations of enclaves
			Self::unregister_timed_out_enclaves();
			if let Some(snapshot_nonce) = <SnapshotNonce<T>>::get() {
				if let Some(snapshot) = <Snapshots<T>>::get(snapshot_nonce.saturating_sub(1)) {
					<IngressMessages<T>>::put(Vec::<
						polkadex_primitives::ingress::IngressMessages<T::AccountId>,
					>::from([
						polkadex_primitives::ingress::IngressMessages::LastestSnapshot(
							snapshot.snapshot_hash,
							snapshot.snapshot_number,
						),
					]));
				} else {
					<IngressMessages<T>>::put(Vec::<
						polkadex_primitives::ingress::IngressMessages<T::AccountId>,
					>::new());
				}
			} else {
				<IngressMessages<T>>::put(Vec::<
					polkadex_primitives::ingress::IngressMessages<T::AccountId>,
				>::new());
			}

			<OnChainEvents<T>>::put(BoundedVec::<
				polkadex_primitives::ocex::OnChainEvents<T::AccountId>,
				OnChainEventsLimit,
			>::default());

			(1000000 as Weight)
				.saturating_add(T::DbWeight::get().reads(2 as Weight))
				.saturating_add(T::DbWeight::get().writes(2 as Weight))
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers a new account in orderbook
		#[pallet::weight(<T as Config>::WeightInfo::register_main_account())]
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

		/// Adds approval from a validator for an enclave's attestation report
		#[pallet::weight(<T as Config>::WeightInfo::add_proxy_account())]
		pub fn approve_enclave_report(
			origin: OriginFor<T>,
			approver: T::OCEXId,
			signature: <T::OCEXId as RuntimeAppPublic>::Signature,
			report: Vec<u8>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// verify if signature from approver along with other params
			ensure!(approver.verify(&report, &signature), Error::<T>::ReportSignatureInvalid);
			if let Some(mut approvers) = <UnverifiedReports<T>>::get(&report) {
				approvers.push(approver);
				let len = approvers.len();
				if len + len / 3 >= Self::authorities().len() {
					// And then register if enough voted
					let account_id = match T::AccountId::decode(&mut &report[368..400]) {
						Ok(aid) => aid,
						Err(_) => {
							//ensure!(false, Error::<T>::SenderIsNotAttestedEnclave);
							T::AccountId::decode(&mut &[0u8; 32].to_vec()[..]).unwrap()
						},
					};
					let next_24h = <timestamp::Pallet<T>>::get()
						.saturating_add(T::Moment::saturated_from(86_400_000u64));
					<RegisteredEnclaves<T>>::mutate(&account_id, |v| {
						*v = Some(T::Moment::saturated_from(next_24h));
					});
					<UnverifiedReports<T>>::remove(report);
					Self::deposit_event(Event::EnclaveRegistered(
						account_id,
						T::Moment::saturated_from(86_400_000u64),
					));
				}
			}
			Ok(())
		}

		/// Adds a proxy account to a pre-registered main acocunt
		#[pallet::weight(<T as Config>::WeightInfo::add_proxy_account())]
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
		#[pallet::weight(100000)]
		pub fn close_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(<TradingPairs<T>>::contains_key(base, quote), Error::<T>::TradingPairNotFound);
			<TradingPairs<T>>::mutate(base, quote, |value| {
				if let Some(trading_pair) = value {
					trading_pair.operational_status = false;
					<IngressMessages<T>>::mutate(|ingress_messages| {
						ingress_messages.push(
							polkadex_primitives::ingress::IngressMessages::CloseTradingPair(
								trading_pair.clone(),
							),
						);
					});
					Self::deposit_event(Event::ShutdownTradingPair { pair: trading_pair.clone() });
				} else {
					//scope never executed, already ensured if trading pair exits above
				}
			});
			Ok(())
		}

		/// Registers a new trading pair
		#[pallet::weight(100000)]
		pub fn open_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(<TradingPairs<T>>::contains_key(base, quote), Error::<T>::TradingPairNotFound);
			//update the operational status of the trading pair as true.
			<TradingPairs<T>>::mutate(base, quote, |value| {
				if let Some(trading_pair) = value {
					trading_pair.operational_status = true;
					<IngressMessages<T>>::mutate(|ingress_messages| {
						ingress_messages.push(
							polkadex_primitives::ingress::IngressMessages::OpenTradingPair(
								trading_pair.clone(),
							),
						);
					});
					Self::deposit_event(Event::OpenTradingPair { pair: trading_pair.clone() });
				} else {
					//scope never executed, already ensured if trading pair exits above
				}
			});
			Ok(())
		}

		/// Registers a new trading pair
		#[pallet::weight(100000)]
		pub fn register_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
			min_order_price: BalanceOf<T>,
			max_order_price: BalanceOf<T>,
			min_order_qty: BalanceOf<T>,
			max_order_qty: BalanceOf<T>,
			price_tick_size: BalanceOf<T>,
			qty_step_size: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				!<TradingPairs<T>>::contains_key(base, quote),
				Error::<T>::TradingPairAlreadyRegistered
			);
			ensure!(
				!<TradingPairs<T>>::contains_key(quote, base),
				Error::<T>::TradingPairAlreadyRegistered
			);

			// We need to also check if provided values are not zero
			ensure!(
				min_order_price.saturated_into::<u128>() > 0 &&
					max_order_price.saturated_into::<u128>() > 0 &&
					min_order_qty.saturated_into::<u128>() > 0 &&
					max_order_qty.saturated_into::<u128>() > 0 &&
					price_tick_size.saturated_into::<u128>() > 0 &&
					qty_step_size.saturated_into::<u128>() > 0,
				Error::<T>::TradingPairConfigCannotBeZero
			);

			// We need to check if the provided parameters are not exceeding 10^27 so that there
			// will not be an overflow upon performing calculations
			ensure!(
				min_order_price.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				max_order_price.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				min_order_qty.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				max_order_qty.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				price_tick_size.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				qty_step_size.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);

			// TODO: Check if base and quote assets are enabled for deposits
			// Decimal::from() here is infallable as we ensure provided parameters do not exceed
			// Decimal::MAX
			let trading_pair_info = TradingPairConfig {
				base_asset: base,
				quote_asset: quote,
				min_price: Decimal::from(min_order_price.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				max_price: Decimal::from(max_order_price.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				price_tick_size: Decimal::from(price_tick_size.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				min_qty: Decimal::from(min_order_qty.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				max_qty: Decimal::from(max_order_qty.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				qty_step_size: Decimal::from(qty_step_size.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				operational_status: true,
			};
			<TradingPairs<T>>::insert(base, quote, trading_pair_info.clone());
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

		/// Updates the trading pair config
		#[pallet::weight(100000)]
		pub fn update_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
			min_order_price: BalanceOf<T>,
			max_order_price: BalanceOf<T>,
			min_order_qty: BalanceOf<T>,
			max_order_qty: BalanceOf<T>,
			price_tick_size: BalanceOf<T>,
			qty_step_size: BalanceOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(base, quote),
				Error::<T>::TradingPairNotRegistered
			);

			// We need to also check if provided values are not zero
			ensure!(
				min_order_price.saturated_into::<u128>() > 0 &&
					max_order_price.saturated_into::<u128>() > 0 &&
					min_order_qty.saturated_into::<u128>() > 0 &&
					max_order_qty.saturated_into::<u128>() > 0 &&
					price_tick_size.saturated_into::<u128>() > 0 &&
					qty_step_size.saturated_into::<u128>() > 0,
				Error::<T>::TradingPairConfigCannotBeZero
			);

			// We need to check if the provided parameters are not exceeding 10^27 so that there
			// will not be an overflow upon performing calculations
			ensure!(
				min_order_price.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				max_order_price.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				min_order_qty.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				max_order_qty.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				price_tick_size.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);
			ensure!(
				qty_step_size.saturated_into::<u128>() <= DEPOSIT_MAX,
				Error::<T>::AmountOverflow
			);

			let trading_pair_info = TradingPairConfig {
				base_asset: base,
				quote_asset: quote,
				min_price: Decimal::from(min_order_price.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				max_price: Decimal::from(max_order_price.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				price_tick_size: Decimal::from(price_tick_size.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				min_qty: Decimal::from(min_order_qty.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				max_qty: Decimal::from(max_order_qty.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				qty_step_size: Decimal::from(qty_step_size.saturated_into::<u128>())
					.div(&Decimal::from(UNIT_BALANCE)),
				operational_status: true,
			};
			<TradingPairs<T>>::insert(base, quote, trading_pair_info.clone());
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(
					polkadex_primitives::ingress::IngressMessages::UpdateTradingPair(
						trading_pair_info,
					),
				);
			});
			Self::deposit_event(Event::TradingPairUpdated { base, quote });
			Ok(())
		}

		/// Deposit Assets to Orderbook
		#[pallet::weight(<T as Config>::WeightInfo::deposit())]
		pub fn deposit(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			// TODO: Check if asset is enabled for deposit

			ensure!(amount.saturated_into::<u128>() <= DEPOSIT_MAX, Error::<T>::AmountOverflow);
			let converted_amount =
				Decimal::from(amount.saturated_into::<u128>()).div(Decimal::from(UNIT_BALANCE));

			// Get Storage Map Value
			if let Some(expected_total_amount) =
				converted_amount.checked_add(Self::total_assets(asset))
			{
				<TotalAssets<T>>::insert(asset, expected_total_amount);
			} else {
				return Err(Error::<T>::AmountOverflow.into())
			}

			Self::transfer_asset(&user, &Self::get_custodian_account(), amount, asset)?;
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ingress::IngressMessages::Deposit(
					user.clone(),
					asset,
					converted_amount,
				));
			});
			Self::deposit_event(Event::DepositSuccessful { user, asset, amount });
			Ok(())
		}

		/// Removes a proxy account from pre-registered main acocunt
		#[pallet::weight(100000)]
		pub fn remove_proxy_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(<Accounts<T>>::contains_key(&main_account), Error::<T>::MainAccountNotFound);
			<Accounts<T>>::try_mutate(&main_account, |account_info| {
				if let Some(account_info) = account_info {
					ensure!(account_info.proxies.len() > 1, Error::<T>::MinimumOneProxyRequired);
					let proxy_positon = account_info
						.proxies
						.iter()
						.position(|account| *account == proxy)
						.ok_or(Error::<T>::ProxyNotFound)?;
					account_info.proxies.remove(proxy_positon);
					<IngressMessages<T>>::mutate(|ingress_messages| {
						ingress_messages.push(
							polkadex_primitives::ingress::IngressMessages::RemoveProxy(
								main_account.clone(),
								proxy.clone(),
							),
						);
					});
				}
				Self::deposit_event(Event::ProxyRemoved { main: main_account.clone(), proxy });
				Ok(())
			})
		}

		/// Extrinsic used by enclave to submit balance snapshot and withdrawal requests
		#[pallet::weight((590_500_000 as Weight).saturating_add(T::DbWeight::get().reads(3 as Weight)).saturating_add(T::DbWeight::get().writes(5 as Weight)))]
		pub fn submit_snapshot(
			origin: OriginFor<T>,
			mut snapshot: EnclaveSnapshot<
				T::AccountId,
				WithdrawalLimit,
				AssetsLimit,
				SnapshotAccLimit,
			>,
			signature: <T as pallet::Config>::Signature,
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
				snapshot.snapshot_number.eq(&(last_snapshot_serial_number + 1)),
				Error::<T>::SnapshotNonceError
			);
			let bytes = snapshot.encode();
			ensure!(
				signature.verify(bytes.as_slice(), &enclave),
				Error::<T>::EnclaveSignatureVerificationFailed
			);
			let current_snapshot_nonce = snapshot.snapshot_number;
			ensure!(
				<OnChainEvents<T>>::try_mutate(|onchain_events| {
					onchain_events.try_push(
						polkadex_primitives::ocex::OnChainEvents::GetStorage(
							polkadex_primitives::ocex::Pallet::OCEX,
							polkadex_primitives::ocex::StorageItem::Withdrawal,
							snapshot.snapshot_number,
						),
					)?;
					Ok::<(), ()>(())
				})
				.is_ok(),
				Error::<T>::OnchainEventsBoundedVecOverflow
			);
			<Withdrawals<T>>::insert(current_snapshot_nonce, snapshot.withdrawals.clone());
			<FeesCollected<T>>::insert(current_snapshot_nonce, snapshot.fees.clone());
			snapshot.withdrawals = Default::default();
			snapshot.fees = Default::default();
			<Snapshots<T>>::insert(current_snapshot_nonce, snapshot.clone());
			<SnapshotNonce<T>>::put(current_snapshot_nonce);
			Ok(())
		}

		// FIXME Only for testing will be removed before mainnet launch
		/// Insert Enclave
		#[doc(hidden)]
		#[pallet::weight(10000 + T::DbWeight::get().writes(1))]
		pub fn insert_enclave(origin: OriginFor<T>, encalve: T::AccountId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let timestamp = <timestamp::Pallet<T>>::get();
			<RegisteredEnclaves<T>>::insert(encalve, timestamp);
			Ok(())
		}

		/// Withdraws Fees Collected
		///
		/// params:  snapshot_number: u32
		#[pallet::weight(100000 + T::DbWeight::get().writes(1))]
		pub fn collect_fees(
			origin: OriginFor<T>,
			snapshot_id: u32,
			beneficiary: T::AccountId,
		) -> DispatchResult {
			// TODO: The caller should be of operational council
			T::GovernanceOrigin::ensure_origin(origin)?;

			let fees: Vec<Fees> = <FeesCollected<T>>::get(snapshot_id).iter().cloned().collect();
			for fee in fees {
				if let Some(converted_fee) =
					fee.amount.saturating_mul(Decimal::from(UNIT_BALANCE)).to_u128()
				{
					Self::transfer_asset(
						&Self::get_custodian_account(),
						&beneficiary,
						converted_fee.saturated_into(),
						fee.asset,
					)?;
				// TODO: Remove the fees from storage if successful
				} else {
					return Err(Error::<T>::FailedToConvertDecimaltoBalance.into())
				}
			}
			Self::deposit_event(Event::FeesClaims { beneficiary, snapshot_id });
			Ok(())
		}

		/// Extrinsic used to shutdown the orderbook
		#[pallet::weight(100000)]
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
		/// params: snapshot_number: u32
		#[pallet::weight((100000 as Weight).saturating_add(T::DbWeight::get().reads(2 as Weight)).saturating_add(T::DbWeight::get().writes(3 as Weight)))]
		pub fn claim_withdraw(
			origin: OriginFor<T>,
			snapshot_id: u32,
			account: T::AccountId,
		) -> DispatchResult {
			// Anyone can claim the withdrawal for any user
			// This is to build services that can enable free withdrawals similar to CEXes.
			let _ = ensure_signed(origin)?;

			let mut withdrawals: WithdrawalsMap<T> = <Withdrawals<T>>::get(snapshot_id);
			ensure!(withdrawals.contains_key(&account), Error::<T>::InvalidWithdrawalIndex);
			if let Some(withdrawal_vector) = withdrawals.get(&account) {
				for x in withdrawal_vector.iter() {
					// TODO: Security: if this fails for a withdrawal in between the iteration, it
					// will double spend.
					if let Some(converted_withdrawal) =
						x.amount.saturating_mul(Decimal::from(UNIT_BALANCE)).to_u128()
					{
						Self::transfer_asset(
							&Self::get_custodian_account(),
							&x.main_account,
							converted_withdrawal.saturated_into(),
							x.asset,
						)?;
					}
				}
				Self::deposit_event(Event::WithdrawalClaimed {
					main: account.clone(),
					withdrawals: withdrawal_vector.to_owned(),
				});
				ensure!(
					<OnChainEvents<T>>::mutate(|onchain_events| {
						onchain_events.try_push(
							polkadex_primitives::ocex::OnChainEvents::OrderBookWithdrawalClaimed(
								snapshot_id,
								account.clone(),
								withdrawal_vector.to_owned(),
							),
						)?;
						Ok::<(), ()>(())
					})
					.is_ok(),
					Error::<T>::OnchainEventsBoundedVecOverflow
				);
			}
			withdrawals.remove(&account);
			<Withdrawals<T>>::insert(snapshot_id, withdrawals);
			Ok(())
		}

		/// In order to register itself - enclave must send it's own report to this extrinsic
		#[pallet::weight(<T as Config>::WeightInfo::register_enclave())]
		pub fn register_enclave(origin: OriginFor<T>, ias_report: Vec<u8>) -> DispatchResult {
			let enclave_signer = ensure_signed(origin)?;

			// Check if enclave_signer is whitelisted
			ensure!(
				<WhitelistedEnclaves<T>>::get(&enclave_signer),
				<Error<T>>::EnclaveNotWhitelisted
			);

			<UnverifiedReports<T>>::insert(ias_report, Self::authorities());

			Ok(())
		}

		/// In order to register itself - enclave account id must be whitelisted and called by
		/// Governance
		#[pallet::weight(<T as Config>::WeightInfo::register_enclave())]
		pub fn whitelist_enclave(
			origin: OriginFor<T>,
			enclave_account_id: T::AccountId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			// It will just overwrite if account_id is already whitelisted
			<WhitelistedEnclaves<T>>::insert(&enclave_account_id, true);
			Self::deposit_event(Event::EnclaveWhitelisted(enclave_account_id));
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// clean-up function - should be called on each block
		fn unregister_timed_out_enclaves() {
			use sp_runtime::traits::CheckedSub;
			let mut enclaves_to_remove = sp_std::vec![];
			let iter = <RegisteredEnclaves<T>>::iter();
			iter.for_each(|(enclave, attested_ts)| {
				let current_timestamp = <timestamp::Pallet<T>>::get();
				// enclave will be removed even if something happens with substraction
				if current_timestamp.checked_sub(&attested_ts).unwrap_or(current_timestamp) >=
					T::MsPerDay::get()
				{
					enclaves_to_remove.push(enclave);
				}
			});
			for enclave in &enclaves_to_remove {
				<RegisteredEnclaves<T>>::remove(enclave);
			}
			Self::deposit_event(Event::EnclaveCleanup(enclaves_to_remove));
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// TODO: Allow all approve_enclave_report to pass through if signer is in the
			// current session and they are not already voted.
			let valid_tx = |provide, rng: u64| {
				ValidTransaction::with_tag_prefix("thea-proc")
					.priority(rng)
					.and_provides([&(&provide, rng.to_be())])
					.longevity(3)
					.propagate(true)
					.build()
			};

			let valid_approve_enclave_report = |approver,
			                                    signature,
			                                    report|
			 -> TransactionValidity {
				if <UnverifiedReports<T>>::get(&report).is_some() &&
					<Authorities<T>>::get().contains(&approver)
				{
					valid_tx(
						(approver.clone(), signature, report),
						<Authorities<T>>::get().iter().position(|n| n == &approver).unwrap_or(100)
							as u64,
					)
				} else {
					InvalidTransaction::Call.into()
				}
			};

			match call {
				Self::Call::approve_enclave_report { approver, signature, report } =>
					valid_approve_enclave_report(approver.clone(), signature, report),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FeesClaims {
			beneficiary: T::AccountId,
			snapshot_id: u32,
		},
		MainAccountRegistered {
			main: T::AccountId,
			proxy: T::AccountId,
		},
		TradingPairRegistered {
			base: AssetId,
			quote: AssetId,
		},
		TradingPairUpdated {
			base: AssetId,
			quote: AssetId,
		},
		DepositSuccessful {
			user: T::AccountId,
			asset: AssetId,
			amount: BalanceOf<T>,
		},
		ShutdownTradingPair {
			pair: TradingPairConfig,
		},
		OpenTradingPair {
			pair: TradingPairConfig,
		},
		EnclaveRegistered(T::AccountId, T::Moment),
		EnclaveWhitelisted(T::AccountId),
		EnclaveCleanup(Vec<T::AccountId>),
		TradingPairIsNotOperational,
		WithdrawalClaimed {
			main: T::AccountId,
			withdrawals: BoundedVec<Withdrawal<T::AccountId>, WithdrawalLimit>,
		},
		NewProxyAdded {
			main: T::AccountId,
			proxy: T::AccountId,
		},
		ProxyRemoved {
			main: T::AccountId,
			proxy: T::AccountId,
		},
		OcexAuthoritySetChanged {
			queued: Vec<T::OCEXId>,
		},
	}

	// A map that has enumerable entries.
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub(super) type Accounts<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		AccountInfo<T::AccountId, ProxyLimit>,
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
		TradingPairConfig,
		OptionQuery,
	>;

	// Snapshots Storage
	#[pallet::storage]
	#[pallet::getter(fn snapshots)]
	pub(super) type Snapshots<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, EnclaveSnapshotType<T>, OptionQuery>;

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
	pub(super) type FeesCollected<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, BoundedVec<Fees, AssetsLimit>, ValueQuery>;

	// Withdrawals mapped by their trading pairs and snapshot numbers
	#[pallet::storage]
	#[pallet::getter(fn withdrawals)]
	pub(super) type Withdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, u32, WithdrawalsMap<T>, ValueQuery>;

	// Whitelisted enclaves
	#[pallet::storage]
	#[pallet::getter(fn whitelisted_enclaves)]
	pub(super) type WhitelistedEnclaves<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

	// Reports in process of approval
	#[pallet::storage]
	pub(super) type UnverifiedReports<T: Config> =
		StorageMap<_, Blake2_128Concat, Vec<u8>, Vec<T::OCEXId>, OptionQuery>;

	// Queue for enclave ingress messages
	#[pallet::storage]
	#[pallet::getter(fn ingress_messages)]
	pub(super) type IngressMessages<T: Config> = StorageValue<
		_,
		Vec<polkadex_primitives::ingress::IngressMessages<T::AccountId>>,
		ValueQuery,
	>;

	// Queue for onchain events
	#[pallet::storage]
	#[pallet::getter(fn onchain_events)]
	pub(super) type OnChainEvents<T: Config> = StorageValue<
		_,
		BoundedVec<polkadex_primitives::ocex::OnChainEvents<T::AccountId>, OnChainEventsLimit>,
		ValueQuery,
	>;

	// Total Assets present in orderbook
	#[pallet::storage]
	#[pallet::getter(fn total_assets)]
	pub(super) type TotalAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId, Decimal, ValueQuery>;

	// Vector of registered enclaves
	#[pallet::storage]
	#[pallet::getter(fn get_registered_enclaves)]
	pub(super) type RegisteredEnclaves<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::Moment, OptionQuery>;

	/// Authorities set of current session
	#[pallet::storage]
	#[pallet::getter(fn authorities)]
	pub(super) type Authorities<T: Config> = StorageValue<_, Vec<T::OCEXId>, ValueQuery>;

	/// Authorities set of next session
	#[pallet::storage]
	#[pallet::getter(fn next_authorities)]
	pub(super) type NextAuthorities<T: Config> = StorageValue<_, Vec<T::OCEXId>, ValueQuery>;

	/// The current validator set id
	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, ocex_primitives::ValidatorSetId, ValueQuery>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	fn change_authorities(new: Vec<T::OCEXId>, queued: Vec<T::OCEXId>) {
		// As in GRANDPA, we trigger a validator set change only if the the validator
		// set has actually changed.
		if new != Self::authorities() {
			<Authorities<T>>::put(&new);

			let next_id = Self::validator_set_id() + 1u64;
			<ValidatorSetId<T>>::put(next_id);

			let log: DigestItem = DigestItem::Consensus(
				OCEX_ENGINE_ID,
				ConsensusLog::AuthoritiesChange(ocex_primitives::ValidatorSet {
					validators: new,
					id: next_id,
				})
				.encode(),
			);
			<frame_system::Pallet<T>>::deposit_log(log);
		}

		<NextAuthorities<T>>::put(&queued);
		Self::deposit_event(Event::OcexAuthoritySetChanged { queued });
	}

	fn initialize_authorities(authorities: &[T::OCEXId]) {
		if authorities.is_empty() {
			return
		}

		assert!(<Authorities<T>>::get().is_empty(), "Authorities are already initialized!");

		<Authorities<T>>::put(authorities);
		<ValidatorSetId<T>>::put(0);
		<NextAuthorities<T>>::put(authorities);
	}

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

	pub fn unverified_reports(verifier: &T::OCEXId) -> bool {
		<UnverifiedReports<T>>::iter_values().any(|validators| validators.contains(verifier))
	}

	/// Return the current active OCEX validator set.
	pub fn validator_set() -> ocex_primitives::ValidatorSet<T::OCEXId> {
		let current_set_id = Self::validator_set_id();
		ocex_primitives::ValidatorSet::<T::OCEXId> {
			validators: Self::authorities(),
			id: current_set_id,
		}
	}

	pub fn get_unverified_reports() -> Vec<(Vec<u8>, Vec<T::OCEXId>)> {
		<UnverifiedReports<T>>::iter().collect()
	}
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::OCEXId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::OCEXId)>,
	{
		let authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		Self::initialize_authorities(&authorities);
	}

	fn on_new_session<'a, I: 'a>(changed: bool, validators: I, queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::OCEXId)>,
	{
		let next_authorities = validators.map(|(_, k)| k).collect::<Vec<_>>();
		let next_queued_authorities = queued_validators.map(|(_, k)| k).collect::<Vec<_>>();

		if next_queued_authorities != next_authorities {
			<NextAuthorities<T>>::put(next_queued_authorities);
		}

		if changed {
			<Authorities<T>>::put(next_authorities);
		}
	}

	fn on_before_session_ending() {
		// TODO (after first iteration): Handle offence reports here
	}

	fn on_disabled(i: u32) {
		// TODO (after first iteration): we need to prevent this validator from approving enclaves
		let log: DigestItem = DigestItem::Consensus(
			OCEX_ENGINE_ID,
			ConsensusLog::<T::OCEXId>::OnDisabled(i as AuthorityIndex).encode(),
		);

		<frame_system::Pallet<T>>::deposit_log(log);
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::OCEXId;
}

impl<T: Config> IsMember<T::OCEXId> for Pallet<T> {
	fn is_member(authority_id: &T::OCEXId) -> bool {
		Self::authorities().iter().any(|id| id == authority_id)
	}
}
