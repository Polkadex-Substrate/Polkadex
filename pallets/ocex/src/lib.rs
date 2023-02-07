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
	BoundedVec,
};
use frame_system::ensure_signed;
use polkadex_primitives::{assets::AssetId, OnChainEventsLimit};

use pallet_timestamp::{self as timestamp};
use sp_runtime::traits::{AccountIdConversion, UniqueSaturatedInto};
use sp_std::prelude::*;
// Re-export pallet items so that they can be accessed from the crate namespace.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::traits::One;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

pub use weights::*;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const DEPOSIT_MAX: u128 = 1_000_000_000_000_000_000_000_000_000;
const WITHDRAWAL_MAX: u128 = 1_000_000_000_000_000_000_000_000_000;
const TRADE_OPERATION_MIN_VALUE: u128 = 10000;

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {
	use core::fmt::Debug;
	// Import various types used to declare pallet in scope.
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_tracing::debug,
		storage::bounded_btree_map::BoundedBTreeMap,
		traits::{
			fungibles::{Create, Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use ias_verify::{verify_ias_report, SgxStatus};
	use polkadex_primitives::{
		assets::AssetId,
		ocex::{AccountInfo, TradingPairConfig},
		snapshot::{EnclaveSnapshot, Fees},
		withdrawal::Withdrawal,
		AssetsLimit, ProxyLimit, SnapshotAccLimit, WithdrawalLimit, UNIT_BALANCE,
	};
	use rust_decimal::{prelude::ToPrimitive, Decimal};
	use sp_runtime::{
		traits::{IdentifyAccount, Verify},
		BoundedBTreeSet, SaturatedConversion,
	};
	use sp_std::vec::Vec;
	use thea_primitives::liquidity::LiquidityModifier;

	pub trait OcexWeightInfo {
		fn register_main_account(_b: u32) -> Weight;
		fn add_proxy_account(x: u32) -> Weight;
		fn close_trading_pair(_x: u32) -> Weight;
		fn open_trading_pair(_x: u32) -> Weight;
		fn register_trading_pair(_x: u32) -> Weight;
		fn update_trading_pair(_x: u32) -> Weight;
		fn deposit(_x: u32) -> Weight;
		fn remove_proxy_account(x: u32) -> Weight;
		fn submit_snapshot() -> Weight;
		fn insert_enclave(_x: u32) -> Weight;
		fn collect_fees(_x: u32) -> Weight;
		fn shutdown() -> Weight;
		fn set_exchange_state(_x: u32) -> Weight;
		fn set_balances(_x: u32) -> Weight;
		fn claim_withdraw(_x: u32) -> Weight;
		fn register_enclave(_x: u32) -> Weight;
		fn allowlist_token(_x: u32) -> Weight;
		fn remove_allowlisted_token(_x: u32) -> Weight;
		fn allowlist_enclave(_x: u32) -> Weight;
		fn update_certificate(_x: u32) -> Weight;
	}

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

	pub struct AllowlistedTokenLimit;
	impl Get<u32> for AllowlistedTokenLimit {
		fn get() -> u32 {
			50 // TODO: Arbitrary value
		}
	}

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
			> + Inspect<<Self as frame_system::Config>::AccountId>
			+ Create<<Self as frame_system::Config>::AccountId>;

		/// Origin that can send orderbook snapshots and withdrawal requests
		type EnclaveOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
		type Public: Clone
			+ PartialEq
			+ IdentifyAccount<AccountId = Self::AccountId>
			+ Debug
			+ parity_scale_codec::Codec
			+ Ord
			+ scale_info::TypeInfo;

		/// A matching `Signature` type.
		type Signature: Verify<Signer = Self::Public>
			+ Clone
			+ PartialEq
			+ Debug
			+ parity_scale_codec::Codec
			+ scale_info::TypeInfo;

		/// Type representing the weight of this pallet
		type WeightInfo: OcexWeightInfo;

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
		/// Account is not registered with the exchange
		AccountNotRegistered,
		InvalidWithdrawalIndex,
		/// Amount within withdrawal can not be converted to Decimal
		InvalidWithdrawalAmount,
		/// The trading pair is not currently Operational
		TradingPairIsNotOperational,
		/// the trading pair is currently in operation
		TradingPairIsNotClosed,
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
		/// Enclave not allowlisted
		EnclaveNotAllowlisted,
		/// Trading Pair is not registed for updating
		TradingPairNotRegistered,
		/// Trading Pair config value cannot be set to zero
		TradingPairConfigCannotBeZero,
		/// Limit reached to add allowlisted token
		AllowlistedTokenLimitReached,
		/// Given token is not allowlisted
		TokenNotAllowlisted,
		/// Given allowlisted token is removed
		AllowlistedTokenRemoved,
		/// Trading Pair config value cannot be set to zero
		TradingPairConfigUnderflow,
		/// Exchange is down
		ExchangeNotOperational,
		/// Unable to transfer fee
		UnableToTransferFee,
		/// Unable to execute collect fees fully
		FeesNotCollectedFully,
		/// Exchange is up
		ExchangeOperational,
		/// Can not write into withdrawal bounded structure
		/// limit reached
		WithdrawalBoundOverflow,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// What to do at the end of each block.
		///
		/// Clean IngressMessages
		fn on_initialize(_n: T::BlockNumber) -> Weight {
			// When block's been initialized - clean up expired registrations of enclaves
			//Self::unregister_timed_out_enclaves();
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
		#[pallet::weight(<T as Config>::WeightInfo::register_main_account(1))]
		pub fn register_main_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			Self::register_user(main_account, proxy)?;
			Ok(())
		}

		/// Adds a proxy account to a pre-registered main acocunt
		#[pallet::weight(<T as Config>::WeightInfo::add_proxy_account(1))]
		pub fn add_proxy_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
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
		#[pallet::weight(<T as Config>::WeightInfo::close_trading_pair(1))]
		pub fn close_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
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
		#[pallet::weight(<T as Config>::WeightInfo::open_trading_pair(1))]
		pub fn open_trading_pair(
			origin: OriginFor<T>,
			base: AssetId,
			quote: AssetId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
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
		#[pallet::weight(<T as Config>::WeightInfo::register_trading_pair(1))]
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
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);

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

			//enclave will only support min volume of 10^-8
			//if trading pairs volume falls below it will pass a UnderFlow Error
			ensure!(
				min_order_price.saturated_into::<u128>() > TRADE_OPERATION_MIN_VALUE &&
					min_order_qty.saturated_into::<u128>() > TRADE_OPERATION_MIN_VALUE &&
					min_order_price
						.saturated_into::<u128>()
						.saturating_mul(min_order_qty.saturated_into::<u128>()) >
						TRADE_OPERATION_MIN_VALUE,
				Error::<T>::TradingPairConfigUnderflow
			);

			// TODO: Check if base and quote assets are enabled for deposits
			// Decimal::from() here is infallable as we ensure provided parameters do not exceed
			// Decimal::MAX
			match (
				Decimal::from(min_order_price.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(max_order_price.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(price_tick_size.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(min_order_qty.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(max_order_qty.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(qty_step_size.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
			) {
				(
					Some(min_price),
					Some(max_price),
					Some(price_tick_size),
					Some(min_qty),
					Some(max_qty),
					Some(qty_step_size),
				) => {
					let trading_pair_info = TradingPairConfig {
						base_asset: base,
						quote_asset: quote,
						min_price,
						max_price,
						price_tick_size,
						min_qty,
						max_qty,
						qty_step_size,
						operational_status: true,
						base_asset_precision: qty_step_size.scale() as u8,
						quote_asset_precision: price_tick_size.scale() as u8,
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
				},
				//passing Underflow error if checked_div fails
				_ => Err(Error::<T>::TradingPairConfigUnderflow.into()),
			}
		}

		/// Updates the trading pair config
		#[pallet::weight(<T as Config>::WeightInfo::update_trading_pair(1))]
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
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
			ensure!(base != quote, Error::<T>::BothAssetsCannotBeSame);
			ensure!(
				<TradingPairs<T>>::contains_key(base, quote),
				Error::<T>::TradingPairNotRegistered
			);
			let is_pair_in_operation = match <TradingPairs<T>>::get(base, quote) {
				Some(config) => config.operational_status,
				None => false,
			};
			ensure!(!is_pair_in_operation, Error::<T>::TradingPairIsNotClosed);
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

			//enclave will only support min volume of 10^-8
			//if trading pairs volume falls below it will pass a UnderFlow Error
			ensure!(
				min_order_price.saturated_into::<u128>() > TRADE_OPERATION_MIN_VALUE &&
					min_order_qty.saturated_into::<u128>() > TRADE_OPERATION_MIN_VALUE &&
					min_order_price
						.saturated_into::<u128>()
						.saturating_mul(min_order_qty.saturated_into::<u128>()) >
						TRADE_OPERATION_MIN_VALUE,
				Error::<T>::TradingPairConfigUnderflow
			);

			match (
				Decimal::from(min_order_price.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(max_order_price.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(price_tick_size.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(min_order_qty.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(max_order_qty.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
				Decimal::from(qty_step_size.saturated_into::<u128>())
					.checked_div(Decimal::from(UNIT_BALANCE)),
			) {
				(
					Some(min_price),
					Some(max_price),
					Some(price_tick_size),
					Some(min_qty),
					Some(max_qty),
					Some(qty_step_size),
				) => {
					let trading_pair_info = TradingPairConfig {
						base_asset: base,
						quote_asset: quote,
						min_price,
						max_price,
						price_tick_size,
						min_qty,
						max_qty,
						qty_step_size,
						operational_status: true,
						base_asset_precision: price_tick_size.scale() as u8, /* scale() can never be                                                    * greater u8::MAX */
						quote_asset_precision: qty_step_size.scale() as u8, /* scale() can never be                                                    * greater than u8::MAX */
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
				},
				_ => Err(Error::<T>::TradingPairConfigUnderflow.into()),
			}
		}

		/// Deposit Assets to Orderbook
		#[pallet::weight(<T as Config>::WeightInfo::deposit(1))]
		pub fn deposit(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			Self::do_deposit(user, asset, amount)?;
			Ok(())
		}

		/// Removes a proxy account from pre-registered main account
		#[pallet::weight(<T as Config>::WeightInfo::remove_proxy_account(1))]
		pub fn remove_proxy_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
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
		#[pallet::weight(<T as Config>::WeightInfo::submit_snapshot())]
		pub fn submit_snapshot(
			origin: OriginFor<T>,
			mut snapshot: EnclaveSnapshot<
				T::AccountId,
				WithdrawalLimit,
				AssetsLimit,
				SnapshotAccLimit,
			>,
			signature: T::Signature,
		) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			ensure!(
				<RegisteredEnclaves<T>>::contains_key(&snapshot.enclave_id),
				Error::<T>::SenderIsNotAttestedEnclave
			);
			ensure!(
				<AllowlistedEnclaves<T>>::get(&snapshot.enclave_id),
				<Error<T>>::EnclaveNotAllowlisted
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
				signature.verify(bytes.as_slice(), &snapshot.enclave_id),
				Error::<T>::EnclaveSignatureVerificationFailed
			);
			let current_snapshot_nonce = snapshot.snapshot_number;
			if snapshot.withdrawals.keys().len() > 0 {
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
			}
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
		#[pallet::weight(<T as Config>::WeightInfo::insert_enclave(1))]
		pub fn insert_enclave(origin: OriginFor<T>, enclave: T::AccountId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let timestamp = <timestamp::Pallet<T>>::get();
			<RegisteredEnclaves<T>>::insert(enclave, timestamp);
			Ok(())
		}

		/// Withdraws Fees Collected
		///
		/// params:  snapshot_number: u32
		#[pallet::weight(<T as Config>::WeightInfo::collect_fees(1))]
		pub fn collect_fees(
			origin: OriginFor<T>,
			snapshot_id: u32,
			beneficiary: T::AccountId,
		) -> DispatchResult {
			// TODO: The caller should be of operational council
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(
				<FeesCollected<T>>::mutate(snapshot_id, |internal_vector| {
					while internal_vector.len() > 0 {
						if let Some(fees) = internal_vector.pop() {
							if let Some(converted_fee) =
								fees.amount.saturating_mul(Decimal::from(UNIT_BALANCE)).to_u128()
							{
								if Self::transfer_asset(
									&Self::get_pallet_account(),
									&beneficiary,
									converted_fee.saturated_into(),
									fees.asset,
								)
								.is_err()
								{
									// Push it back inside the internal vector
									// The above function call will only fail if the beneficiary has
									// balance below existential deposit requirements
									internal_vector.try_push(fees).unwrap_or_default();
									return Err(Error::<T>::UnableToTransferFee)
								}
							} else {
								// Push it back inside the internal vector
								internal_vector.try_push(fees).unwrap_or_default();
								return Err(Error::<T>::FailedToConvertDecimaltoBalance)
							}
						}
					}
					Ok(())
				})
				.is_ok(),
				Error::<T>::FeesNotCollectedFully
			);
			Self::deposit_event(Event::FeesClaims { beneficiary, snapshot_id });
			Ok(())
		}

		/// Extrinsic used to shutdown the orderbook
		#[pallet::weight(<T as Config>::WeightInfo::shutdown())]
		pub fn shutdown(origin: OriginFor<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<ExchangeState<T>>::put(false);
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(polkadex_primitives::ingress::IngressMessages::Shutdown);
			});
			Ok(())
		}

		///This extrinsic will pause/resume the exchange according to flag
		/// If flag is set to false it will stop the exchange
		/// If flag is set to true it will resume the exchange
		#[pallet::weight(<T as Config>::WeightInfo::set_exchange_state(1))]
		pub fn set_exchange_state(origin: OriginFor<T>, state: bool) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<ExchangeState<T>>::put(state);

			//SetExchangeState Ingress message store in queue
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages
					.push(polkadex_primitives::ingress::IngressMessages::SetExchangeState(state))
			});

			Self::deposit_event(Event::ExchangeStateUpdated(state));
			Ok(())
		}

		/// Sends the changes required in balances for list of users with a particular asset
		#[pallet::weight(<T as Config>::WeightInfo::set_balances(change_in_balances.len().saturated_into()))]
		pub fn set_balances(
			origin: OriginFor<T>,
			change_in_balances: BoundedVec<
				polkadex_primitives::ingress::HandleBalance<T::AccountId>,
				polkadex_primitives::ingress::HandleBalanceLimit,
			>,
		) -> DispatchResult {
			// Check if governance called the extrinsic
			T::GovernanceOrigin::ensure_origin(origin)?;

			// Check if exchange is pause
			ensure!(!Self::orderbook_operational_state(), Error::<T>::ExchangeOperational);

			//Pass the vec as ingress message
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(
					polkadex_primitives::ingress::IngressMessages::SetFreeReserveBalanceForAccounts(
						change_in_balances,
					),
				);
			});
			Ok(())
		}

		/// Withdraws user balance
		///
		/// params: snapshot_number: u32
		/// account: AccountId
		#[pallet::weight(<T as Config>::WeightInfo::claim_withdraw(1))]
		pub fn claim_withdraw(
			origin: OriginFor<T>,
			snapshot_id: u32,
			account: T::AccountId,
		) -> DispatchResultWithPostInfo {
			// Anyone can claim the withdrawal for any user
			// This is to build services that can enable free withdrawals similar to CEXes.
			let _ = ensure_signed(origin)?;
			// This vector will keep track of withdrawals processed already
			let mut processed_withdrawals = vec![];
			let mut failed_withdrawals = vec![];
			ensure!(
				<Withdrawals<T>>::contains_key(snapshot_id),
				Error::<T>::InvalidWithdrawalIndex
			);
			// This entire block of code is put inside ensure as some of the nested functions will
			// return Err
			<Withdrawals<T>>::mutate(snapshot_id, |btree_map| {
				// Get mutable reference to the withdrawals vector
				if let Some(withdrawal_vector) = btree_map.get_mut(&account) {
					while withdrawal_vector.len() > 0 {
						// Perform pop operation to ensure we do not leave any withdrawal left
						// for a double spend
						if let Some(withdrawal) = withdrawal_vector.pop() {
							if let Some(converted_withdrawal) = withdrawal
								.amount
								.saturating_mul(Decimal::from(UNIT_BALANCE))
								.to_u128()
							{
								if Self::transfer_asset(
									&Self::get_pallet_account(),
									&withdrawal.main_account,
									converted_withdrawal.saturated_into(),
									withdrawal.asset,
								)
								.is_ok()
								{
									processed_withdrawals.push(withdrawal.to_owned());
								} else {
									// Storing the failed withdrawals back into the storage item
									failed_withdrawals.push(withdrawal.to_owned());
									Self::deposit_event(Event::WithdrawalFailed(
										withdrawal.to_owned(),
									));
								}
							} else {
								return Err(Error::<T>::InvalidWithdrawalAmount)
							}
						}
					}
					// Not removing key from BtreeMap so that failed withdrawals can still be
					// tracked
					btree_map
						.try_insert(
							account.clone(),
							failed_withdrawals
								.try_into()
								.map_err(|_| Error::<T>::WithdrawalBoundOverflow)?,
						)
						.map_err(|_| Error::<T>::WithdrawalBoundOverflow)?;
					Ok(())
				} else {
					// This allows us to ensure we do not have someone with an invalid account
					Err(Error::<T>::InvalidWithdrawalIndex)
				}
			})?;
			if !processed_withdrawals.is_empty() {
				Self::deposit_event(Event::WithdrawalClaimed {
					main: account.clone(),
					withdrawals: processed_withdrawals.clone(),
				});
				<OnChainEvents<T>>::mutate(|onchain_events| {
					onchain_events
						.try_push(
							polkadex_primitives::ocex::OnChainEvents::OrderBookWithdrawalClaimed(
								snapshot_id,
								account.clone(),
								processed_withdrawals
									.clone()
									.try_into()
									.map_err(|_| Error::<T>::WithdrawalBoundOverflow)?,
							),
						)
						.map_err(|_| Error::<T>::WithdrawalBoundOverflow)?;
					Ok::<(), Error<T>>(())
				})?;
				Ok(Pays::No.into())
			} else {
				// If someone withdraws nothing successfully - should pay for such transaction
				Ok(Pays::Yes.into())
			}
		}

		/// In order to register itself - enclave must send it's own report to this extrinsic
		#[pallet::weight(<T as Config>::WeightInfo::register_enclave(1))]
		pub fn register_enclave(origin: OriginFor<T>, ias_report: Vec<u8>) -> DispatchResult {
			let _ = ensure_signed(origin)?;

			// this step is required for runtime-benchmarks
			let cv: u64 = <CertificateValidity<T>>::get();
			let report = verify_ias_report(&ias_report, cv)
				.map_err(|_| Error::<T>::RemoteAttestationVerificationFailed)?;

			ensure!(
				(report.status == SgxStatus::Ok) |
					(report.status == SgxStatus::ConfigurationNeeded),
				<Error<T>>::InvalidSgxReportStatus
			);

			let enclave_signer = T::AccountId::decode(&mut &report.pubkey[..])
				.map_err(|_| Error::<T>::SenderIsNotAttestedEnclave)?;

			ensure!(
				enclave_signer != T::AccountId::decode(&mut [0u8; 32].as_slice()).unwrap(),
				<Error<T>>::SenderIsNotAttestedEnclave
			);

			<RegisteredEnclaves<T>>::mutate(&enclave_signer, |v| {
				*v = T::Moment::saturated_from(report.timestamp);
			});
			Self::deposit_event(Event::EnclaveRegistered(enclave_signer));
			debug!("registered enclave at time =>{:?}", report.timestamp);
			Ok(())
		}

		/// Allowlist Token
		#[pallet::weight(<T as Config>::WeightInfo::allowlist_token(1))]
		pub fn allowlist_token(origin: OriginFor<T>, token: AssetId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let mut allowlisted_tokens = <AllowlistedToken<T>>::get();
			allowlisted_tokens
				.try_insert(token)
				.map_err(|_| Error::<T>::AllowlistedTokenLimitReached)?;
			<AllowlistedToken<T>>::put(allowlisted_tokens);
			Self::deposit_event(Event::<T>::TokenAllowlisted(token));
			Ok(())
		}

		/// Remove Allowlisted Token
		#[pallet::weight(<T as Config>::WeightInfo::remove_allowlisted_token(1))]
		pub fn remove_allowlisted_token(origin: OriginFor<T>, token: AssetId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let mut allowlisted_tokens = <AllowlistedToken<T>>::get();
			allowlisted_tokens.remove(&token);
			<AllowlistedToken<T>>::put(allowlisted_tokens);
			Self::deposit_event(Event::<T>::AllowlistedTokenRemoved(token));
			Ok(())
		}

		/// In order to register itself - enclave account id must be allowlisted and called by
		/// Governance
		#[pallet::weight(<T as Config>::WeightInfo::allowlist_enclave(1))]
		pub fn allowlist_enclave(
			origin: OriginFor<T>,
			enclave_account_id: T::AccountId,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			// It will just overwrite if account_id is already allowlisted
			<AllowlistedEnclaves<T>>::insert(&enclave_account_id, true);
			Self::deposit_event(Event::EnclaveAllowlisted(enclave_account_id));
			Ok(())
		}

		/// Extrinsic to update ExchangeState
		#[pallet::weight(<T as Config>::WeightInfo::update_certificate(1))]
		pub fn update_certificate(
			origin: OriginFor<T>,
			certificate_valid_until: u64,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<CertificateValidity<T>>::put(certificate_valid_until);
			Ok(())
		}
	}

	impl<T: Config> LiquidityModifier for Pallet<T> {
		type AssetId = AssetId;
		type AccountId = T::AccountId;

		fn on_deposit(
			account: Self::AccountId,
			asset: Self::AssetId,
			balance: u128,
		) -> DispatchResult {
			Self::do_deposit(account, asset, balance.saturated_into())?;
			Ok(())
		}
		fn on_withdraw(
			account: Self::AccountId,
			proxy_account: Self::AccountId,
			asset: Self::AssetId,
			balance: u128,
			do_force_withdraw: bool,
		) -> DispatchResult {
			Self::withdrawal_from_orderbook(
				account,
				proxy_account,
				asset,
				balance.saturated_into(),
				do_force_withdraw,
			)?;
			Ok(())
		}
		fn on_register(main_account: Self::AccountId, proxy: Self::AccountId) -> DispatchResult {
			Self::register_user(main_account, proxy)?;
			Ok(())
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn set_exchange_state_to_true() -> DispatchResult {
			<ExchangeState<T>>::put(true);
			Ok(())
		}

		#[cfg(feature = "runtime-benchmarks")]
		fn allowlist_and_create_token(account: Self::AccountId, token: u128) -> DispatchResult {
			let asset: AssetId = AssetId::asset(token);
			let mut allowlisted_tokens = <AllowlistedToken<T>>::get();
			allowlisted_tokens
				.try_insert(asset)
				.map_err(|_| Error::<T>::AllowlistedTokenLimitReached)?;
			<AllowlistedToken<T>>::put(allowlisted_tokens);
			let amount = BalanceOf::<T>::decode(&mut &(u128::MAX).to_le_bytes()[..])
				.map_err(|_| Error::<T>::FailedToConvertDecimaltoBalance)?;
			//create asset and mint into it.
			T::OtherAssets::create(
				token,
				Self::get_pallet_account(),
				true,
				BalanceOf::<T>::one().unique_saturated_into(),
			)?;
			T::OtherAssets::mint_into(token, &account.clone(), amount)?;
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		// clean-up function - should be called on each block
		#[allow(dead_code)]
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

		pub fn do_deposit(
			user: T::AccountId,
			asset: AssetId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
			ensure!(<AllowlistedToken<T>>::get().contains(&asset), Error::<T>::TokenNotAllowlisted);
			// Check if account is registered
			ensure!(<Accounts<T>>::contains_key(&user), Error::<T>::AccountNotRegistered);
			ensure!(amount.saturated_into::<u128>() <= DEPOSIT_MAX, Error::<T>::AmountOverflow);
			let converted_amount = Decimal::from(amount.saturated_into::<u128>())
				.checked_div(Decimal::from(UNIT_BALANCE))
				.ok_or(Error::<T>::FailedToConvertDecimaltoBalance)?;

			Self::transfer_asset(&user, &Self::get_pallet_account(), amount, asset)?;
			// Get Storage Map Value
			if let Some(expected_total_amount) =
				converted_amount.checked_add(Self::total_assets(asset))
			{
				<TotalAssets<T>>::insert(asset, expected_total_amount);
			} else {
				return Err(Error::<T>::AmountOverflow.into())
			}

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

		pub fn register_user(main_account: T::AccountId, proxy: T::AccountId) -> DispatchResult {
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
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

		pub fn withdrawal_from_orderbook(
			user: T::AccountId,
			proxy_account: T::AccountId,
			asset: AssetId,
			amount: BalanceOf<T>,
			do_force_withdraw: bool,
		) -> DispatchResult {
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
			ensure!(<AllowlistedToken<T>>::get().contains(&asset), Error::<T>::TokenNotAllowlisted);
			// Check if account is registered
			ensure!(<Accounts<T>>::contains_key(&user), Error::<T>::AccountNotRegistered);
			ensure!(amount.saturated_into::<u128>() <= WITHDRAWAL_MAX, Error::<T>::AmountOverflow);
			let converted_amount = Decimal::from(amount.saturated_into::<u128>())
				.checked_div(Decimal::from(UNIT_BALANCE))
				.ok_or(Error::<T>::FailedToConvertDecimaltoBalance)?;
			<IngressMessages<T>>::mutate(|ingress_messages| {
				ingress_messages.push(
					polkadex_primitives::ingress::IngressMessages::DirectWithdrawal(
						proxy_account,
						asset,
						converted_amount,
						do_force_withdraw,
					),
				);
			});
			Self::deposit_event(Event::WithdrawFromOrderbook(user, asset, amount));
			Ok(())
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
		EnclaveRegistered(T::AccountId),
		EnclaveAllowlisted(T::AccountId),
		EnclaveCleanup(Vec<T::AccountId>),
		TradingPairIsNotOperational,
		WithdrawalClaimed {
			main: T::AccountId,
			withdrawals: Vec<Withdrawal<T::AccountId>>,
		},
		NewProxyAdded {
			main: T::AccountId,
			proxy: T::AccountId,
		},
		ProxyRemoved {
			main: T::AccountId,
			proxy: T::AccountId,
		},
		/// TokenAllowlisted
		TokenAllowlisted(AssetId),
		/// AllowlistedTokenRemoved
		AllowlistedTokenRemoved(AssetId),
		/// Withdrawal failed
		WithdrawalFailed(Withdrawal<T::AccountId>),
		/// Exchange state has been updated
		ExchangeStateUpdated(bool),
		/// Withdraw Assets from Orderbook
		WithdrawFromOrderbook(T::AccountId, AssetId, BalanceOf<T>),
	}

	///Allowlisted tokens
	#[pallet::storage]
	#[pallet::getter(fn get_allowlisted_token)]
	pub(super) type AllowlistedToken<T: Config> =
		StorageValue<_, BoundedBTreeSet<AssetId, AllowlistedTokenLimit>, ValueQuery>;

	///CertificateValidity
	#[pallet::storage]
	#[pallet::getter(fn get_certificate_validation_time)]
	pub(super) type CertificateValidity<T: Config> = StorageValue<_, u64, ValueQuery>;

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

	// Allowlisted enclaves
	#[pallet::storage]
	#[pallet::getter(fn allowlisted_enclaves)]
	pub(super) type AllowlistedEnclaves<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, bool, ValueQuery>;

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
		StorageMap<_, Blake2_128Concat, T::AccountId, T::Moment, ValueQuery>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config> Pallet<T> {
	/// Returns the AccountId to hold user funds, note this account has no private keys and
	/// can accessed using on-chain logic.
	fn get_pallet_account() -> T::AccountId {
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
