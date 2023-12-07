// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

//! # Off Chain EXchange Pallet.
//!
//! The OCEX pallet is the foundation for the fund security. This pallet handles all the critical
//! operational tasks.

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(unused_crate_dependencies)]

extern crate core;

use frame_support::{
	dispatch::DispatchResult,
	pallet_prelude::{InvalidTransaction, TransactionValidity, ValidTransaction, Weight},
	traits::{
		fungibles::{Inspect, Mutate},
		tokens::{Fortitude, Preservation},
		Currency, ExistenceRequirement, Get, OneSessionHandler,
	},
	BoundedVec,
};
use frame_system::ensure_signed;
use pallet_timestamp as timestamp;
use parity_scale_codec::Encode;
use polkadex_primitives::{assets::AssetId, AccountId, UNIT_BALANCE};
use rust_decimal::Decimal;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	traits::{AccountIdConversion, UniqueSaturatedInto},
	Percent, SaturatedConversion,
};
use sp_std::{ops::Div, prelude::*};
// Re-export pallet items so that they can be accessed from the crate namespace.
use frame_system::pallet_prelude::BlockNumberFor;
use orderbook_primitives::{
	types::{AccountAsset, TradingPair},
	SnapshotSummary, ValidatorSet, GENESIS_AUTHORITY_SET_ID,
};
pub use pallet::*;
use polkadex_primitives::ocex::TradingPairConfig;
#[cfg(feature = "runtime-benchmarks")]
use sp_runtime::traits::One;
use sp_std::vec::Vec;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

pub const OCEX: KeyTypeId = KeyTypeId(*b"ocex");

pub mod sr25519 {
	mod app_sr25519 {
		use super::super::OCEX;
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, OCEX);
	}

	sp_application_crypto::with_pair! {
		/// An OCEX keypair using sr25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// An OCEX signature using sr25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// An OCEX identifier using sr25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}

pub mod aggregator;
#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod lmp;
pub mod rpc;
mod settlement;
mod snapshot;
pub mod storage;
pub mod validator;

/// A type alias for the balance type from this pallet's point of view.
type BalanceOf<T> =
	<<T as Config>::NativeCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

const DEPOSIT_MAX: u128 = 1_000_000_000_000_000_000_000_000_000;
const WITHDRAWAL_MAX: u128 = 1_000_000_000_000_000_000_000_000_000;
const TRADE_OPERATION_MIN_VALUE: u128 = 10000;

/// Weight abstraction required for "ocex" pallet.
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
	fn collect_fees(_x: u32) -> Weight;
	fn set_exchange_state(_x: u32) -> Weight;
	fn set_balances(_x: u32) -> Weight;
	fn claim_withdraw(_x: u32) -> Weight;
	fn allowlist_token(_x: u32) -> Weight;
	fn remove_allowlisted_token(_x: u32) -> Weight;
	fn set_snapshot() -> Weight;
	fn whitelist_orderbook_operator() -> Weight;
}

// Definition of the pallet logic, to be aggregated at runtime definition through
// `construct_runtime`.
#[allow(clippy::too_many_arguments)]
#[frame_support::pallet]
pub mod pallet {

	use sp_std::collections::btree_map::BTreeMap;
	// Import various types used to declare pallet in scope.
	use super::*;
	use crate::validator::WORKER_STATUS;
	use frame_support::{
		pallet_prelude::*,
		traits::{
			fungibles::{Create, Inspect, Mutate},
			Currency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::{offchain::SendTransactionTypes, pallet_prelude::*};
	use liquidity::LiquidityModifier;
	use orderbook_primitives::{Fees, ObCheckpointRaw, SnapshotSummary};
	use polkadex_primitives::{
		assets::AssetId,
		ocex::{AccountInfo, TradingPairConfig},
		withdrawal::Withdrawal,
		ProxyLimit, UNIT_BALANCE,
	};
	use rust_decimal::{prelude::ToPrimitive, Decimal};
	use sp_application_crypto::RuntimeAppPublic;
	use sp_runtime::{
		offchain::storage::StorageValueRef, traits::BlockNumberProvider, BoundedBTreeSet,
		SaturatedConversion,
	};
	use sp_std::vec::Vec;

	type WithdrawalsMap<T> = BTreeMap<
		<T as frame_system::Config>::AccountId,
		Vec<Withdrawal<<T as frame_system::Config>::AccountId>>,
	>;

	pub struct AllowlistedTokenLimit;

	impl Get<u32> for AllowlistedTokenLimit {
		fn get() -> u32 {
			50 // TODO: Arbitrary value
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_: TransactionSource, call: &Self::Call) -> TransactionValidity {
			sp_runtime::print("Validating unsigned transactions...");
			match call {
				Call::submit_snapshot { summary, signatures } =>
					Self::validate_snapshot(summary, signatures),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	/// Our pallet's configuration trait. All our types and constants go in here. If the
	/// pallet is dependent on specific other pallets, then their configuration traits
	/// should be added to our implied traits list.
	///
	/// `frame_system::Config` should always be included.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + timestamp::Config + SendTransactionTypes<Call<Self>>
	{
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

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
		type EnclaveOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// The identifier type for an authority.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Ord
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen;

		/// Governance Origin
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;

		/// Type representing the weight of this pallet
		type WeightInfo: OcexWeightInfo;
	}

	// Simple declaration of the `Pallet` type. It is placeholder we use to implement traits and
	// method.
	#[pallet::pallet]
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
		/// Proxy is already in use
		ProxyAlreadyRegistered,
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
		/// Unable to aggregrate the signature
		InvalidSignatureAggregation,
		/// Unable to get signer index
		SignerIndexNotFound,
		/// Snapshot in invalid state
		InvalidSnapshotState,
		/// AccountId cannot be decoded
		AccountIdCannotBeDecoded,
		/// Withdrawal called with in disputation period is live
		WithdrawStillInDisputationPeriod,
		/// Snapshot is disputed by validators
		WithdrawBelongsToDisputedSnapshot,
		///Cannot query SnapshotDisputeCloseBlockMap
		SnapshotDisputeCloseBlockStorageQueryError,
		///Cannot find close block for snapshot
		CannotFindCloseBlockForSnapshot,
		/// Dispute Interval not set
		DisputeIntervalNotSet,
		/// Worker not Idle
		WorkerNotIdle,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			let len = <OnChainEvents<T>>::get().len();
			if len > 0 {
				<OnChainEvents<T>>::kill();
				Weight::default()
					.saturating_add(T::DbWeight::get().reads(1)) // we've read length
					.saturating_add(T::DbWeight::get().writes(1)) // kill places None once into Value
			} else {
				Weight::zero().saturating_add(T::DbWeight::get().reads(1)) // justh length was read
			}
		}

		fn offchain_worker(block_number: BlockNumberFor<T>) {
			log::debug!(target:"ocex", "offchain worker started");

			match Self::run_on_chain_validation(block_number) {
				Ok(exit_flag) => {
					// If exit flag is false, then another worker is online
					if !exit_flag {
						return
					}
				},
				Err(err) => {
					log::error!(target:"ocex","OCEX worker error: {}",err);
				},
			}
			// Set worker status to false
			let s_info = StorageValueRef::persistent(&WORKER_STATUS);
			s_info.set(&false);
			log::debug!(target:"ocex", "OCEX worker exiting...");
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Registers a new account in orderbook.
		#[pallet::call_index(0)]
		#[pallet::weight(< T as Config >::WeightInfo::register_main_account(1))]
		pub fn register_main_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			Self::register_user(main_account, proxy)?;
			Ok(())
		}

		/// Adds a proxy account to a pre-registered main account.
		#[pallet::call_index(1)]
		#[pallet::weight(< T as Config >::WeightInfo::add_proxy_account(1))]
		pub fn add_proxy_account(origin: OriginFor<T>, proxy: T::AccountId) -> DispatchResult {
			let main_account = ensure_signed(origin)?;
			ensure!(Self::orderbook_operational_state(), Error::<T>::ExchangeNotOperational);
			ensure!(<Accounts<T>>::contains_key(&main_account), Error::<T>::MainAccountNotFound);
			// Avoid duplicate Proxy accounts
			ensure!(!<Proxies<T>>::contains_key(&proxy), Error::<T>::ProxyAlreadyRegistered);
			if let Some(mut account_info) = <Accounts<T>>::get(&main_account) {
				ensure!(
					account_info.add_proxy(proxy.clone()).is_ok(),
					Error::<T>::ProxyLimitExceeded
				);
				let current_blk = frame_system::Pallet::<T>::current_block_number();
				<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
					ingress_messages.push(polkadex_primitives::ingress::IngressMessages::AddProxy(
						main_account.clone(),
						proxy.clone(),
					));
				});
				<Accounts<T>>::insert(&main_account, account_info);
				<Proxies<T>>::insert(&proxy, main_account.clone());
				Self::deposit_event(Event::NewProxyAdded { main: main_account, proxy });
			}
			Ok(())
		}

		/// Closes trading pair.
		#[pallet::call_index(2)]
		#[pallet::weight(< T as Config >::WeightInfo::close_trading_pair(1))]
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
					let current_blk = frame_system::Pallet::<T>::current_block_number();
					<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
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

		/// Opens a new trading pair.
		#[pallet::call_index(3)]
		#[pallet::weight(< T as Config >::WeightInfo::open_trading_pair(1))]
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
					let current_blk = frame_system::Pallet::<T>::current_block_number();
					<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
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

		/// Registers a new trading pair.
		#[pallet::call_index(4)]
		#[pallet::weight(< T as Config >::WeightInfo::register_trading_pair(1))]
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
					let current_blk = frame_system::Pallet::<T>::current_block_number();
					<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
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

		/// Updates the trading pair configuration.
		#[pallet::call_index(5)]
		#[pallet::weight(< T as Config >::WeightInfo::update_trading_pair(1))]
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
						base_asset_precision: price_tick_size.scale().saturated_into(),
						quote_asset_precision: qty_step_size.scale().saturated_into(),
					};

					<TradingPairs<T>>::insert(base, quote, trading_pair_info.clone());
					let current_blk = frame_system::Pallet::<T>::current_block_number();
					<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
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

		/// Deposit Assets to the Orderbook.
		#[pallet::call_index(6)]
		#[pallet::weight(< T as Config >::WeightInfo::deposit(1))]
		pub fn deposit(
			origin: OriginFor<T>,
			asset: AssetId,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			Self::do_deposit(user, asset, amount)?;
			Ok(())
		}

		/// Removes a proxy account from pre-registered main account.
		#[pallet::call_index(7)]
		#[pallet::weight(< T as Config >::WeightInfo::remove_proxy_account(1))]
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
					let current_blk = frame_system::Pallet::<T>::current_block_number();
					<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
						ingress_messages.push(
							polkadex_primitives::ingress::IngressMessages::RemoveProxy(
								main_account.clone(),
								proxy.clone(),
							),
						);
					});
					<Proxies<T>>::remove(proxy.clone());
					Self::deposit_event(Event::ProxyRemoved { main: main_account.clone(), proxy });
				}
				Ok(())
			})
		}

		/// Sets snapshot id as current. Callable by governance only.
		///
		/// # Parameters
		///
		/// * `origin`: signed member of T::GovernanceOrigin.
		/// * `new_snapshot_id`: u64 id of new *current* snapshot.
		#[pallet::call_index(8)]
		#[pallet::weight(< T as Config >::WeightInfo::set_snapshot())]
		pub fn set_snapshot(origin: OriginFor<T>, new_snapshot_id: u64) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<SnapshotNonce<T>>::put(new_snapshot_id);
			Ok(())
		}

		/// Collects withdraws fees.
		///
		/// # Parameters
		///
		/// * `snapshot_id`: Snapshot identifier.
		/// * `beneficiary`: Receiving fee account identifier.
		#[pallet::call_index(11)]
		#[pallet::weight(< T as Config >::WeightInfo::collect_fees(1))]
		pub fn collect_fees(
			origin: OriginFor<T>,
			snapshot_id: u64,
			beneficiary: T::AccountId,
		) -> DispatchResult {
			// TODO: The caller should be of operational council
			T::GovernanceOrigin::ensure_origin(origin)?;

			ensure!(
				<FeesCollected<T>>::mutate(snapshot_id, |internal_vector| {
					while !internal_vector.is_empty() {
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
									internal_vector.push(fees);
									return Err(Error::<T>::UnableToTransferFee)
								}
							} else {
								// Push it back inside the internal vector
								internal_vector.push(fees);
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

		/// This extrinsic will pause/resume the exchange according to flag.
		/// If flag is set to false it will stop the exchange.
		/// If flag is set to true it will resume the exchange.
		#[pallet::call_index(12)]
		#[pallet::weight(< T as Config >::WeightInfo::set_exchange_state(1))]
		pub fn set_exchange_state(origin: OriginFor<T>, state: bool) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<ExchangeState<T>>::put(state);
			let current_blk = frame_system::Pallet::<T>::current_block_number();
			//SetExchangeState Ingress message store in queue
			<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
				ingress_messages
					.push(polkadex_primitives::ingress::IngressMessages::SetExchangeState(state))
			});

			Self::deposit_event(Event::ExchangeStateUpdated(state));
			Ok(())
		}

		/// Sends the changes required in balances for list of users with a particular asset.
		#[pallet::call_index(13)]
		#[pallet::weight(< T as Config >::WeightInfo::set_balances(1))]
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
			let current_blk = frame_system::Pallet::<T>::current_block_number();
			//Pass the vec as ingress message
			<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
				ingress_messages.push(
					polkadex_primitives::ingress::IngressMessages::SetFreeReserveBalanceForAccounts(
						change_in_balances,
					),
				);
			});
			Ok(())
		}

		/// Withdraws user balance.
		///
		/// # Parameters
		///
		/// * `snapshot_id`: Key of the withdrawal in the storage map.
		/// * `account`: Account identifier.
		#[pallet::call_index(14)]
		#[pallet::weight(< T as Config >::WeightInfo::claim_withdraw(1))]
		pub fn claim_withdraw(
			origin: OriginFor<T>,
			snapshot_id: u64,
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
					while !withdrawal_vector.is_empty() {
						// Perform pop operation to ensure we do not leave any withdrawal left
						// for a double spend
						if let Some(withdrawal) = withdrawal_vector.pop() {
							if Self::on_idle_withdrawal_processor(withdrawal.clone()) {
								processed_withdrawals.push(withdrawal.to_owned());
							} else {
								// Storing the failed withdrawals back into the storage item
								failed_withdrawals.push(withdrawal.to_owned());
								Self::deposit_event(Event::WithdrawalFailed(withdrawal.to_owned()));
							}
						} else {
							return Err(Error::<T>::InvalidWithdrawalAmount)
						}
					}
					// Not removing key from BtreeMap so that failed withdrawals can still be
					// tracked
					btree_map.insert(account.clone(), failed_withdrawals);
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
					onchain_events.push(
						polkadex_primitives::ocex::OnChainEvents::OrderBookWithdrawalClaimed(
							snapshot_id,
							account.clone(),
							processed_withdrawals,
						),
					);
				});
				Ok(Pays::No.into())
			} else {
				// If someone withdraws nothing successfully - should pay for such transaction
				Ok(Pays::Yes.into())
			}
		}

		/// Allowlist Token
		#[pallet::call_index(15)]
		#[pallet::weight(< T as Config >::WeightInfo::allowlist_token(1))]
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
		#[pallet::call_index(16)]
		#[pallet::weight(< T as Config >::WeightInfo::remove_allowlisted_token(1))]
		pub fn remove_allowlisted_token(origin: OriginFor<T>, token: AssetId) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			let mut allowlisted_tokens = <AllowlistedToken<T>>::get();
			allowlisted_tokens.remove(&token);
			<AllowlistedToken<T>>::put(allowlisted_tokens);
			Self::deposit_event(Event::<T>::AllowlistedTokenRemoved(token));
			Ok(())
		}

		/// Submit Snapshot Summary
		#[pallet::call_index(17)]
		#[pallet::weight(< T as Config >::WeightInfo::submit_snapshot())]
		pub fn submit_snapshot(
			origin: OriginFor<T>,
			summary: SnapshotSummary<T::AccountId>,
			_signatures: Vec<(u16, <T::AuthorityId as RuntimeAppPublic>::Signature)>,
		) -> DispatchResult {
			ensure_none(origin)?;
			// TODO: Process egress messages from summary.
			if !summary.withdrawals.is_empty() {
				let withdrawal_map = Self::create_withdrawal_tree(&summary.withdrawals);
				<Withdrawals<T>>::insert(summary.snapshot_id, withdrawal_map);
				<FeesCollected<T>>::insert(summary.snapshot_id, summary.get_fees());
				<OnChainEvents<T>>::mutate(|onchain_events| {
					onchain_events.push(
						polkadex_primitives::ocex::OnChainEvents::OrderbookWithdrawalProcessed(
							summary.snapshot_id,
							summary.withdrawals.clone(),
						),
					);
				});
			}
			let id = summary.snapshot_id;
			<SnapshotNonce<T>>::put(id);
			<Snapshots<T>>::insert(id, summary);
			Self::deposit_event(Event::<T>::SnapshotProcessed(id));
			Ok(())
		}

		/// Submit Snapshot Summary
		#[pallet::call_index(18)]
		#[pallet::weight(< T as Config >::WeightInfo::whitelist_orderbook_operator())]
		pub fn whitelist_orderbook_operator(
			origin: OriginFor<T>,
			operator_public_key: sp_core::ecdsa::Public,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<OrderbookOperatorPublicKey<T>>::put(operator_public_key);
			Self::deposit_event(Event::<T>::OrderbookOperatorKeyWhitelisted(operator_public_key));
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
			let asset: AssetId = AssetId::Asset(token);
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
			let current_blk = frame_system::Pallet::<T>::current_block_number();
			<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
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
			// Avoid duplicate Proxy accounts
			ensure!(!<Proxies<T>>::contains_key(&proxy), Error::<T>::ProxyAlreadyRegistered);

			let mut account_info = AccountInfo::new(main_account.clone());
			ensure!(account_info.add_proxy(proxy.clone()).is_ok(), Error::<T>::ProxyLimitExceeded);
			<Accounts<T>>::insert(&main_account, account_info);

			let current_blk = frame_system::Pallet::<T>::current_block_number();
			<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
				ingress_messages.push(polkadex_primitives::ingress::IngressMessages::RegisterUser(
					main_account.clone(),
					proxy.clone(),
				));
			});
			<Proxies<T>>::insert(&proxy, main_account.clone());
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
			let current_blk = frame_system::Pallet::<T>::current_block_number();
			<IngressMessages<T>>::mutate(current_blk, |ingress_messages| {
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

		fn create_withdrawal_tree(
			pending_withdrawals: impl AsRef<[Withdrawal<T::AccountId>]>,
		) -> WithdrawalsMap<T> {
			let mut withdrawal_map: WithdrawalsMap<T> = WithdrawalsMap::<T>::new();
			for withdrawal in pending_withdrawals.as_ref() {
				let recipient_account: T::AccountId = withdrawal.main_account.clone();
				if let Some(pending_withdrawals) = withdrawal_map.get_mut(&recipient_account) {
					pending_withdrawals.push(withdrawal.to_owned())
				} else {
					let pending_withdrawals = sp_std::vec![withdrawal.to_owned()];
					withdrawal_map.insert(recipient_account, pending_withdrawals);
				}
			}
			withdrawal_map
		}

		/// Performs actual transfer of assets from pallet account to target destination
		/// Used to finalize withdrawals in extrinsic or on_idle
		fn on_idle_withdrawal_processor(
			withdrawal: Withdrawal<<T as frame_system::Config>::AccountId>,
		) -> bool {
			if let Some(converted_withdrawal) =
				withdrawal.amount.saturating_mul(Decimal::from(UNIT_BALANCE)).to_u128()
			{
				Self::transfer_asset(
					&Self::get_pallet_account(),
					&withdrawal.main_account,
					converted_withdrawal.saturated_into(),
					withdrawal.asset,
				)
				.is_ok()
			} else {
				false
			}
		}

		/// Collects onchain registered main and proxy accounts
		/// for each of main accounts collects balances from offchain storage
		/// adds other required for recovery properties
		/// Returned tuple resembles `orderbook_primitives::recovery::ObRecoveryState`
		/// FIXME: use solid type here instead of tuple
		pub fn get_ob_recover_state() -> Result<
			(
				u64,
				BTreeMap<AccountId, Vec<AccountId>>,
				BTreeMap<AccountAsset, Decimal>,
				u32,
				u64,
				u64,
			),
			DispatchError,
		> {
			let account_id =
				<Accounts<T>>::iter().fold(vec![], |mut ids_accum, (acc, acc_info)| {
					ids_accum.push((acc.clone(), acc_info.proxies));
					ids_accum
				});

			let mut balances: BTreeMap<AccountAsset, Decimal> = BTreeMap::new();
			let mut account_ids: BTreeMap<AccountId, Vec<AccountId>> = BTreeMap::new();
			// all offchain balances for main accounts
			for account in account_id {
				let main = Self::transform_account(account.0)?;
				let b = Self::get_offchain_balance(&main)?;
				for (asset, balance) in b.into_iter() {
					balances.insert(AccountAsset { main: main.clone(), asset }, balance);
				}
				let proxies = account.1.into_iter().try_fold(vec![], |mut accum, proxy| {
					accum.push(Self::transform_account(proxy)?);
					Ok::<Vec<AccountId>, DispatchError>(accum)
				})?;
				account_ids.insert(main, proxies);
			}

			let state_info = Self::get_state_info().map_err(|_err| DispatchError::Corruption)?;
			let last_processed_block_number = state_info.last_block;
			let worker_nonce = state_info.worker_nonce;
			let snapshot_id = state_info.snapshot_id;
			let state_change_id = state_info.stid;

			Ok((
				snapshot_id,
				account_ids,
				balances,
				last_processed_block_number,
				state_change_id,
				worker_nonce,
			))
		}

		/// Fetch checkpoint for recovery
		pub fn fetch_checkpoint() -> Result<ObCheckpointRaw, DispatchError> {
			log::debug!(target:"ocex", "fetch_checkpoint called");
			let account_id =
				<Accounts<T>>::iter().fold(vec![], |mut ids_accum, (acc, acc_info)| {
					ids_accum.push((acc.clone(), acc_info.proxies));
					ids_accum
				});

			let mut balances: BTreeMap<AccountAsset, Decimal> = BTreeMap::new();
			// all offchain balances for main accounts
			for account in account_id {
				let main = Self::transform_account(account.0)?;
				let b = Self::get_offchain_balance(&main)?;
				for (asset, balance) in b.into_iter() {
					balances.insert(AccountAsset { main: main.clone(), asset }, balance);
				}
			}
			let state_info = Self::get_state_info().map_err(|_err| DispatchError::Corruption)?;
			let last_processed_block_number = state_info.last_block;
			let snapshot_id = state_info.snapshot_id;
			let state_change_id = state_info.stid;
			log::debug!(target:"ocex", "fetch_checkpoint returning");
			Ok(ObCheckpointRaw::new(
				snapshot_id,
				balances,
				last_processed_block_number,
				state_change_id,
			))
		}

		/// Fetches balance of given `AssetId` for given `AccountId` from offchain storage
		/// If nothing found - returns `Decimal::Zero`
		pub fn get_balance(from: T::AccountId, of: AssetId) -> Result<Decimal, DispatchError> {
			Ok(Self::get_offchain_balance(&Self::transform_account(from)?)
				.unwrap_or_else(|_| BTreeMap::new())
				.get(&of)
				.unwrap_or(&Decimal::ZERO)
				.to_owned())
		}

		// Converts `T::AccountId` into `polkadex_primitives::AccountId`
		fn transform_account(
			account: T::AccountId,
		) -> Result<polkadex_primitives::AccountId, DispatchError> {
			Decode::decode(&mut &account.encode()[..])
				.map_err(|_| Error::<T>::AccountIdCannotBeDecoded.into())
		}
	}

	/// Events are a simple means of reporting specific conditions and
	/// circumstances that have happened that users, Dapps and/or chain explorers would find
	/// interesting and otherwise difficult to detect.
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		RebroadcastTriggered,
		SnapshotProcessed(u64),
		UserActionsBatchSubmitted(u64),
		FeesClaims {
			beneficiary: T::AccountId,
			snapshot_id: u64,
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
		/// DisputePeriod has been updated
		DisputePeriodUpdated(BlockNumberFor<T>),
		/// Withdraw Assets from Orderbook
		WithdrawFromOrderbook(T::AccountId, AssetId, BalanceOf<T>),
		/// Orderbook Operator Key Whitelisted
		OrderbookOperatorKeyWhitelisted(sp_core::ecdsa::Public),
	}

	///Allowlisted tokens
	#[pallet::storage]
	#[pallet::getter(fn get_allowlisted_token)]
	pub(super) type AllowlistedToken<T: Config> =
		StorageValue<_, BoundedBTreeSet<AssetId, AllowlistedTokenLimit>, ValueQuery>;

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

	// Proxy to main account map
	#[pallet::storage]
	#[pallet::getter(fn proxies)]
	pub(super) type Proxies<T: Config> =
		StorageMap<_, Blake2_128Concat, T::AccountId, T::AccountId, OptionQuery>;

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
	pub type Snapshots<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, SnapshotSummary<T::AccountId>, OptionQuery>;

	// Snapshots Nonce
	#[pallet::storage]
	#[pallet::getter(fn snapshot_nonce)]
	pub type SnapshotNonce<T: Config> = StorageValue<_, u64, ValueQuery>;

	// Exchange Operation State
	#[pallet::storage]
	#[pallet::getter(fn orderbook_operational_state)]
	pub(super) type ExchangeState<T: Config> = StorageValue<_, bool, ValueQuery>;

	// Fees collected
	#[pallet::storage]
	#[pallet::getter(fn fees_collected)]
	pub(super) type FeesCollected<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, Vec<Fees>, ValueQuery>;

	// Withdrawals mapped by their trading pairs and snapshot numbers
	#[pallet::storage]
	#[pallet::getter(fn withdrawals)]
	pub(super) type Withdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, u64, WithdrawalsMap<T>, ValueQuery>;

	// Queue for enclave ingress messages
	#[pallet::storage]
	#[pallet::getter(fn ingress_messages)]
	pub(super) type IngressMessages<T: Config> = StorageMap<
		_,
		Identity,
		BlockNumberFor<T>,
		Vec<polkadex_primitives::ingress::IngressMessages<T::AccountId>>,
		ValueQuery,
	>;

	// Queue for onchain events
	#[pallet::storage]
	#[pallet::getter(fn onchain_events)]
	pub(super) type OnChainEvents<T: Config> =
		StorageValue<_, Vec<polkadex_primitives::ocex::OnChainEvents<T::AccountId>>, ValueQuery>;

	// Total Assets present in orderbook
	#[pallet::storage]
	#[pallet::getter(fn total_assets)]
	pub(super) type TotalAssets<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetId, Decimal, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_authorities)]
	pub(super) type Authorities<T: Config> = StorageMap<
		_,
		Identity,
		orderbook_primitives::ValidatorSetId,
		ValidatorSet<T::AuthorityId>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_next_authorities)]
	pub(super) type NextAuthorities<T: Config> =
		StorageValue<_, ValidatorSet<T::AuthorityId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn validator_set_id)]
	pub(super) type ValidatorSetId<T: Config> =
		StorageValue<_, orderbook_primitives::ValidatorSetId, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_orderbook_operator_public_key)]
	pub(super) type OrderbookOperatorPublicKey<T: Config> =
		StorageValue<_, sp_core::ecdsa::Public, OptionQuery>;

	#[crate::pallet::storage]
	#[crate::pallet::getter(fn lmp_epoch)]
	pub(super) type LMPEpoch<T: crate::pallet::Config> = StorageValue<_, u32, ValueQuery>;
}

// The main implementation block for the pallet. Functions here fall into three broad
// categories:
// - Public interface. These are functions that are `pub` and generally fall into inspector
// functions that do not write to storage and operation functions that do.
// - Private functions. These are your usual private utilities unavailable to other pallets.
impl<T: Config + frame_system::offchain::SendTransactionTypes<Call<T>>> Pallet<T> {
	pub fn validate_snapshot(
		snapshot_summary: &SnapshotSummary<T::AccountId>,
		signatures: &Vec<(u16, <T::AuthorityId as RuntimeAppPublic>::Signature)>,
	) -> TransactionValidity {
		sp_runtime::print("Validating submit_snapshot....");

		// Verify if snapshot is already processed
		if <SnapshotNonce<T>>::get().saturating_add(1) != snapshot_summary.snapshot_id {
			return InvalidTransaction::Custom(10).into()
		}

		// Check if this validator was part of that authority set
		let authorities = <Authorities<T>>::get(snapshot_summary.validator_set_id).validators;

		//Check threshold

		const MAJORITY: u8 = 67;
		let p = Percent::from_percent(MAJORITY);
		let threshold = p * authorities.len();

		if threshold > signatures.len() {
			return InvalidTransaction::Custom(11).into()
		}

		// Check signatures
		for (index, signature) in signatures {
			match authorities.get(*index as usize) {
				None => return InvalidTransaction::Custom(12).into(),
				Some(auth) =>
					if !auth.verify(&snapshot_summary.encode(), signature) {
						return InvalidTransaction::Custom(12).into()
					},
			}
		}

		sp_runtime::print("submit_snapshot validated!");
		ValidTransaction::with_tag_prefix("orderbook")
			.and_provides([&snapshot_summary.state_hash])
			.longevity(10)
			.propagate(true)
			.build()
	}

	pub fn validator_set() -> ValidatorSet<T::AuthorityId> {
		let id = Self::validator_set_id();
		<Authorities<T>>::get(id)
	}

	// Returns all main accounts and corresponding proxies for it at this point in time
	pub fn get_all_accounts_and_proxies() -> Vec<(T::AccountId, Vec<T::AccountId>)> {
		<Accounts<T>>::iter()
			.map(|(main, info)| (main, info.proxies.to_vec()))
			.collect::<Vec<(T::AccountId, Vec<T::AccountId>)>>()
	}

	/// Returns a vector of allowlisted asset IDs.
	///
	/// # Returns
	/// `Vec<AssetId>`: A vector of allowlisted asset IDs.
	pub fn get_allowlisted_assets() -> Vec<AssetId> {
		<AllowlistedToken<T>>::get().iter().copied().collect::<Vec<AssetId>>()
	}

	/// Returns the AccountId to hold user funds, note this account has no private keys and
	/// can accessed using on-chain logic.
	fn get_pallet_account() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	pub fn read_trading_pair_configs() -> Vec<(TradingPair, TradingPairConfig)> {
		let iterator = <TradingPairs<T>>::iter();
		let mut configs = Vec::new();
		for (base, quote, config) in iterator {
			configs.push((TradingPair { base, quote }, config))
		}
		configs
	}

	fn transfer_asset(
		payer: &T::AccountId,
		payee: &T::AccountId,
		amount: BalanceOf<T>,
		asset: AssetId,
	) -> DispatchResult {
		match asset {
			AssetId::Polkadex => {
				T::NativeCurrency::transfer(
					payer,
					payee,
					amount.unique_saturated_into(),
					ExistenceRequirement::KeepAlive,
				)?;
			},
			AssetId::Asset(id) => {
				T::OtherAssets::transfer(
					id,
					payer,
					payee,
					amount.unique_saturated_into(),
					Preservation::Preserve,
				)?;
			},
		}
		Ok(())
	}

	fn get_onchain_balance(asset: AssetId) -> Decimal {
		let balance = match asset {
			AssetId::Polkadex => T::NativeCurrency::free_balance(&Self::get_pallet_account()),
			AssetId::Asset(id) => T::OtherAssets::reducible_balance(
				id,
				&Self::get_pallet_account(),
				Preservation::Expendable,
				Fortitude::Force,
			),
		};

		// div will not panic since denominator is a constant
		Decimal::from(balance.saturated_into::<u128>()).div(Decimal::from(UNIT_BALANCE))
	}
}

impl<T: Config> sp_application_crypto::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(authorities: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
	{
		let authorities = authorities.map(|(_, k)| k).collect::<Vec<_>>();
		<Authorities<T>>::insert(
			GENESIS_AUTHORITY_SET_ID,
			ValidatorSet::new(authorities, GENESIS_AUTHORITY_SET_ID),
		);
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, authorities: I, queued_authorities: I)
	where
		I: Iterator<Item = (&'a T::AccountId, Self::Key)>,
	{
		let next_authorities = authorities.map(|(_, k)| k).collect::<Vec<_>>();
		let next_queued_authorities = queued_authorities.map(|(_, k)| k).collect::<Vec<_>>();

		let id = Self::validator_set_id();
		let new_id = id + 1u64;

		<Authorities<T>>::insert(new_id, ValidatorSet::new(next_authorities, new_id));
		<NextAuthorities<T>>::put(ValidatorSet::new(next_queued_authorities, new_id + 1));
		<ValidatorSetId<T>>::put(new_id);
	}

	fn on_disabled(_i: u32) {}
}
