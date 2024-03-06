// This file is part of Polkadex.

// Copyright (C) 2020-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! XCM Helper Pallet
//!
//! The XCM Helper Pallet provides functionality to handle XCM Messages. Also it implements multiple
//! traits required by XCM Pallets.
//!
//! - [`Config`]
//! - [`Call`]
//! - [`Pallet`]
//!
//! ## Overview
//!
//! XCM Helper Pallet provides following functionalities:-
//!
//! - Handling withdrawal requests from Relayers.
//! - Managing Thea Public Key.
//! - Parachain asset management.
//! - Executing Withdrawal request every block.
//!
//! ## Terminology
//!
//! - **Thea key** Thea Key is Multi-party ECDSA Public Key which has access to transfer funds from
//!   Polkadex Sovereign Accounts to Others on Native/Foreign Chain using XCMP.
//!
//! - **WithdrawalExecutionBlockDiff** Delays in Blocks after which Pending withdrawal will be
//!   executed.
//!
//! - **ParachainAsset** Type using which native Parachain will identify assets from foregin
//!   Parachain.
//!
//! ### Implementations
//! The XCM Helper pallet provides implementations for the following traits. If these traits provide
//! the functionality that you need, then you can avoid coupling with the XCM Helper pallet.
//!
//! -[`TransactAsset`]: Used by XCM Executor to deposit, withdraw and transfer native/non-native
//! asset on Native Chain. -[`AssetIdConverter`]: Converts Assets id from Multilocation Format to
//! Local Asset Id and vice-versa.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//! - `withdraw_asset` - Transfers Assets from Polkadex Sovereign Account to Others on
//!   native/non-native parachains using XCMP.
//! - `deposit_asset` - Generate Ingress Message for new Deposit.
//! - `transfer_asset` - Transfers Asset from source account to destination account.
//!
//! ### Supported Origins
//! - `AssetCreateUpdateOrigin` - Origin which has access to Create Asset.
//!
//! ### Public Functions
//! - `handle_deposit` - Handles deposits from foreign chain.
//! - `generate_asset_id_for_parachain` - Retrieves the existing asset ID for given assetid or
//!   generates and stores a new asset ID.
//! - `block_by_ele` - Blocks Transaction to be Executed.
//! - `convert_asset_id_to_location` - Converts asset_id to XCM::MultiLocation.
//! - `convert_location_to_asset_id` - Converts Multilocation to u128.
//! - `insert_pending_withdrawal` - Stores provided withdraw in a "PendingWithdrawals" collectiom.
//! - `multi_location_to_account_converter` - Resolves "AccountId" based on provided MultiLocatiom.
//!
//! ### Public Inspection functions - Immutable (accessors)
//! - `get_pallet_account` - Returns Pallet Id.
//! - `get_destination_account` - Converts Multilocation to AccountId.
//! - `is_polkadex_parachain_destination` - Checks if destination address belongs to native
//!   parachain or not.
//! - `is_parachain_asset` - Checks if given asset is native asset or not.
//! - `get_amount` - Converts XCM::Fungibility into u128
//!
//! ### Storage Items
//! - `PendingWithdrawals` - Stores all pending withdrawal.
//! - `FailedWithdrawals` - Stores failed withdrawals which failed during execution.
//! - `ParachainAssets` - Stores assets mapping from u128 asset to multi asset.
//! - `WhitelistedTokens` - Stores whitelisted Tokens.
//! -
//! # Events
//! - `AssetDeposited` - Asset Deposited from XCM.
//! - `AssetWithdrawn` - Asset burned/locked from native Parachain.
//! - `TheaAssetCreated` - New Asset Created.
//! - `TokenWhitelistedForXcm` - Token Whitelisted For Xcm Token.
//! - `XcmFeeTransferred` - Xcm Fee Transferred.
//! - `NativeAssetIdMappingRegistered` - Native asset id mapping is registered.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::weights::{constants::WEIGHT_REF_TIME_PER_SECOND, Weight};
pub use pallet::*;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

/// We allow for 0.5 of a second of compute with a 12 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = Weight::from_parts(
	WEIGHT_REF_TIME_PER_SECOND.saturating_div(2),
	cumulus_primitives_core::relay_chain::MAX_POV_SIZE as u64,
);

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::RawOrigin,
		pallet_prelude::*,
		sp_runtime::traits::AccountIdConversion,
		traits::{
			fungible::{Inspect as InspectNative, Mutate as MutateNative},
			fungibles::Inspect,
			tokens::{Fortitude, Preservation},
		},
		PalletId,
		__private::log,
	};
	use frame_system::pallet_prelude::*;

	use polkadex_primitives::Resolver;
	use sp_core::sp_std;
	use sp_runtime::{traits::Convert, SaturatedConversion};

	use crate::MAXIMUM_BLOCK_WEIGHT;
	use sp_std::{boxed::Box, vec, vec::Vec};
	use thea_primitives::{
		types::{Deposit, Withdraw},
		Network, TheaIncomingExecutor, TheaOutgoingExecutor,
	};
	use xcm::{
		latest::{
			Error as XcmError, Fungibility, Junction, Junctions, MultiAsset, MultiAssets,
			MultiLocation, XcmContext,
		},
		prelude::Parachain,
		v3::AssetId,
		VersionedMultiAssets, VersionedMultiLocation,
	};
	use xcm_executor::{
		traits::{ConvertLocation as MoreConvert, TransactAsset},
		Assets,
	};

	pub trait XcmHelperWeightInfo {
		fn whitelist_token(_b: u32) -> Weight;
		fn remove_whitelisted_token(_b: u32) -> Weight;
		fn transfer_fee(b: u32) -> Weight;
	}

	pub trait AssetIdConverter {
		/// Converts AssetId to MultiLocation
		fn convert_asset_id_to_location(asset_id: u128) -> Option<MultiLocation>;
		/// Converts Location to AssetId
		fn convert_location_to_asset_id(location: MultiLocation) -> Option<u128>;
	}

	pub trait WhitelistedTokenHandler {
		/// Check if token is whitelisted
		fn check_whitelisted_token(asset_id: u128) -> bool;
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + orml_xtokens::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an
		/// event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Multilocation to AccountId Convert
		type AccountIdConvert: MoreConvert<Self::AccountId>;
		/// Assets
		type Assets: frame_support::traits::tokens::fungibles::Mutate<Self::AccountId>
			+ frame_support::traits::tokens::fungibles::Create<Self::AccountId>
			+ frame_support::traits::tokens::fungibles::Inspect<Self::AccountId>;
		/// Asset Id
		type AssetId: Member
			+ Parameter
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ Into<<<Self as Config>::Assets as Inspect<Self::AccountId>>::AssetId>
			+ From<u128>
			+ Into<u128>;
		/// Balances Pallet
		type Currency: frame_support::traits::tokens::fungible::Mutate<Self::AccountId>
			+ frame_support::traits::tokens::fungible::Inspect<Self::AccountId>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;
		/// Message Executor
		type Executor: thea_primitives::TheaOutgoingExecutor;
		/// Pallet Id
		#[pallet::constant]
		type AssetHandlerPalletId: Get<PalletId>;
		/// Pallet Id
		#[pallet::constant]
		type WithdrawalExecutionBlockDiff: Get<BlockNumberFor<Self>>;
		/// PDEX Asset ID
		#[pallet::constant]
		type ParachainId: Get<u32>;
		#[pallet::constant]
		type SubstrateNetworkId: Get<u8>;
		/// Native Asset Id
		#[pallet::constant]
		type NativeAssetId: Get<Self::AssetId>;
		/// Weight Info
		type WeightInfo: XcmHelperWeightInfo;
	}

	/// Pending Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_pending_withdrawals)]
	pub(super) type PendingWithdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Vec<Withdraw>, ValueQuery>;

	/// Failed Withdrawals
	#[pallet::storage]
	#[pallet::getter(fn get_failed_withdrawals)]
	pub(super) type FailedWithdrawals<T: Config> =
		StorageMap<_, Blake2_128Concat, BlockNumberFor<T>, Vec<Withdraw>, ValueQuery>;

	/// Asset mapping from u128 asset to multi asset.
	#[pallet::storage]
	#[pallet::getter(fn assets_mapping)]
	pub type ParachainAssets<T: Config> = StorageMap<_, Identity, u128, AssetId, OptionQuery>;

	/// Whitelist Tokens
	#[pallet::storage]
	#[pallet::getter(fn get_whitelisted_tokens)]
	pub type WhitelistedTokens<T: Config> = StorageValue<_, Vec<u128>, ValueQuery>;

	/// Nonce used to generate randomness for txn id
	#[pallet::storage]
	#[pallet::getter(fn randomness_nonce)]
	pub type RandomnessNonce<T: Config> = StorageValue<_, u32, ValueQuery>;

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Asset Deposited from XCM
		/// parameters. [recipient, multiasset, asset_id]
		AssetDeposited(Box<MultiLocation>, Box<MultiAsset>, u128),
		AssetWithdrawn(T::AccountId, Box<MultiAsset>),
		/// New Asset Created [asset_id]
		TheaAssetCreated(u128),
		/// Token Whitelisted For Xcm [token]
		TokenWhitelistedForXcm(u128),
		/// Xcm Fee Transferred [recipient, amount]
		XcmFeeTransferred(T::AccountId, u128),
		/// Native asset id mapping is registered
		NativeAssetIdMappingRegistered(u128, Box<AssetId>),
		/// Whitelisted Token removed
		WhitelistedTokenRemoved(u128),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Unable to generate asset
		AssetGenerationFailed,
		/// Index not found
		IndexNotFound,
		/// Identifier Length Mismatch
		IdentifierLengthMismatch,
		/// AssetId Abstract Not Handled
		AssetIdAbstractNotHandled,
		/// Pending withdrawal Limit Reached
		PendingWithdrawalsLimitReached,
		/// Token is already Whitelisted
		TokenIsAlreadyWhitelisted,
		/// Whitelisted Tokens limit reached
		WhitelistedTokensLimitReached,
		/// Unable to Decode
		UnableToDecode,
		/// Failed To Push Pending Withdrawal
		FailedToPushPendingWithdrawal,
		/// Unable to Convert to Multi location
		UnableToConvertToMultiLocation,
		/// Unable to Convert to Account
		UnableToConvertToAccount,
		/// Unable to get Assets
		UnableToGetAssets,
		/// Unable to get Deposit Amount
		UnableToGetDepositAmount,
		/// Withdrawal Execution Failed
		WithdrawalExecutionFailed,
		/// Token Is Not Whitelisted
		TokenIsNotWhitelisted,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(n: BlockNumberFor<T>) -> Weight {
			let mut failed_withdrawal: Vec<Withdraw> = Vec::default();
			<PendingWithdrawals<T>>::mutate(n, |withdrawals| {
				while let Some(withdrawal) = withdrawals.pop() {
					if !withdrawal.is_blocked {
						let destination = match VersionedMultiLocation::decode(
							&mut &withdrawal.destination[..],
						) {
							Ok(dest) => dest,
							Err(_) => {
								failed_withdrawal.push(withdrawal);
								log::error!(target:"xcm-helper","Withdrawal failed: Not able to decode destination");
								continue;
							},
						};
						if !Self::is_polkadex_parachain_destination(&destination) {
							if let Some(asset) = Self::assets_mapping(withdrawal.asset_id) {
								let multi_asset = MultiAsset {
									id: asset,
									fun: Fungibility::Fungible(withdrawal.amount),
								};
								let pallet_account: T::AccountId =
									T::AssetHandlerPalletId::get().into_account_truncating();
								// Mint
								if Self::resolver_deposit(
									withdrawal.asset_id.into(),
									withdrawal.amount,
									&pallet_account,
									pallet_account.clone(),
									1u128,
									pallet_account.clone(),
								)
								.is_err()
								{
									failed_withdrawal.push(withdrawal.clone());
									log::error!(target:"xcm-helper","Withdrawal failed: Not able to mint token");
								};
								if orml_xtokens::module::Pallet::<T>::transfer_multiassets(
									RawOrigin::Signed(
										T::AssetHandlerPalletId::get().into_account_truncating(),
									)
									.into(),
									Box::new(multi_asset.into()),
									0,
									Box::new(destination.clone()),
									cumulus_primitives_core::WeightLimit::Unlimited,
								)
								.is_err()
								{
									failed_withdrawal.push(withdrawal.clone());
									log::error!(target:"xcm-helper","Withdrawal failed: Not able to make xcm calls");
								}
							} else {
								failed_withdrawal.push(withdrawal)
							}
						} else if Self::handle_deposit(withdrawal.clone(), destination).is_err() {
							failed_withdrawal.push(withdrawal);
							log::error!(target:"xcm-helper","Withdrawal failed: Not able to handle dest");
						}
					} else {
						failed_withdrawal.push(withdrawal);
						log::error!(target:"xcm-helper","Withdrawal failed: Withdrawal is blocked");
					}
				}
			});
			// Only update the storage if vector is not empty
			if !failed_withdrawal.is_empty() {
				<FailedWithdrawals<T>>::insert(n, failed_withdrawal);
			}
			// TODO: We are currently over estimating the weight here to 1/4th of total block time
			// 	Need a better way to estimate this hook
			MAXIMUM_BLOCK_WEIGHT.saturating_div(4)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Whitelists Token .
		///
		/// # Parameters
		///
		/// * `token`: Token to be whitelisted.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::whitelist_token(1))]
		pub fn whitelist_token(origin: OriginFor<T>, token: AssetId) -> DispatchResult {
			let _ = ensure_signed(origin)?;
			let token = Self::generate_asset_id_for_parachain(token);
			let mut whitelisted_tokens = <WhitelistedTokens<T>>::get();
			ensure!(!whitelisted_tokens.contains(&token), Error::<T>::TokenIsAlreadyWhitelisted);
			let pallet_account: T::AccountId =
				T::AssetHandlerPalletId::get().into_account_truncating();
			Self::resolve_create(token.into(), pallet_account, 1u128)?;
			whitelisted_tokens.push(token);
			<WhitelistedTokens<T>>::put(whitelisted_tokens);
			Self::deposit_event(Event::<T>::TokenWhitelistedForXcm(token));
			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::remove_whitelisted_token(1))]
		pub fn remove_whitelisted_token(
			origin: OriginFor<T>,
			token_to_be_removed: AssetId,
		) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let token_to_be_removed = Self::generate_asset_id_for_parachain(token_to_be_removed);
			let mut whitelisted_tokens = <WhitelistedTokens<T>>::get();
			let index = whitelisted_tokens
				.iter()
				.position(|token| *token == token_to_be_removed)
				.ok_or(Error::<T>::TokenIsNotWhitelisted)?;
			whitelisted_tokens.remove(index);
			<WhitelistedTokens<T>>::put(whitelisted_tokens);
			Self::deposit_event(Event::<T>::WhitelistedTokenRemoved(token_to_be_removed));
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::transfer_fee(1))]
		pub fn transfer_fee(origin: OriginFor<T>, to: T::AccountId) -> DispatchResult {
			T::AssetCreateUpdateOrigin::ensure_origin(origin)?;
			let from = T::AssetHandlerPalletId::get().into_account_truncating();
			let amount =
				T::Currency::reducible_balance(&from, Preservation::Preserve, Fortitude::Polite);
			T::Currency::transfer(&from, &to, amount, Preservation::Protect)?;
			Self::deposit_event(Event::<T>::XcmFeeTransferred(to, amount.saturated_into()));
			Ok(())
		}
	}

	impl<T: Config> Convert<u128, Option<MultiLocation>> for Pallet<T> {
		fn convert(asset_id: u128) -> Option<MultiLocation> {
			Self::convert_asset_id_to_location(asset_id)
		}
	}

	impl<T: Config> TransactAsset for Pallet<T> {
		/// Generate Ingress Message for new Deposit
		fn deposit_asset(
			what: &MultiAsset,
			who: &MultiLocation,
			_context: &XcmContext,
		) -> xcm::latest::Result {
			// Create approved deposit
			let MultiAsset { id, fun } = what;
			let recipient =
				T::AccountIdConvert::convert_location(who).ok_or(XcmError::FailedToDecode)?;
			let amount: u128 = Self::get_amount(fun).ok_or(XcmError::Trap(101))?;
			let asset_id = Self::generate_asset_id_for_parachain(*id);
			let deposit: Deposit<T::AccountId> = Deposit {
				id: Self::new_random_id(),
				recipient,
				asset_id,
				amount,
				extra: Vec::new(),
			};

			let parachain_network_id = T::SubstrateNetworkId::get();
			T::Executor::execute_withdrawals(parachain_network_id, sp_std::vec![deposit].encode())
				.map_err(|_| XcmError::Trap(102))?;
			Self::deposit_event(Event::<T>::AssetDeposited(
				Box::new(*who),
				Box::new(what.clone()),
				asset_id,
			));
			Ok(())
		}

		/// Burns/Lock asset from provided account.
		//TODO: Check for context
		fn withdraw_asset(
			what: &MultiAsset,
			who: &MultiLocation,
			_context: Option<&XcmContext>,
		) -> sp_std::result::Result<Assets, XcmError> {
			let MultiAsset { id: _, fun } = what;
			let who = T::AccountIdConvert::convert_location(who).ok_or(XcmError::FailedToDecode)?;
			let amount: u128 = Self::get_amount(fun).ok_or(XcmError::Trap(101))?;
			let asset_id = Self::generate_asset_id_for_parachain(what.id);
			let pallet_account: T::AccountId =
				T::AssetHandlerPalletId::get().into_account_truncating();
			Self::resolver_withdraw(asset_id.into(), amount.saturated_into(), &who, pallet_account)
				.map_err(|_| XcmError::Trap(25))?;
			Ok(what.clone().into())
		}

		/// Transfers Asset from source account to destination account
		fn transfer_asset(
			asset: &MultiAsset,
			from: &MultiLocation,
			to: &MultiLocation,
			_context: &XcmContext,
		) -> sp_std::result::Result<Assets, XcmError> {
			let MultiAsset { id, fun } = asset;
			let from =
				T::AccountIdConvert::convert_location(from).ok_or(XcmError::FailedToDecode)?;
			let to = T::AccountIdConvert::convert_location(to).ok_or(XcmError::FailedToDecode)?;
			let amount: u128 = Self::get_amount(fun).ok_or(XcmError::Trap(101))?;
			let asset_id = Self::generate_asset_id_for_parachain(*id);
			Self::resolve_transfer(asset_id.into(), &from, &to, amount)
				.map_err(|_| XcmError::Trap(102))?;
			Ok(asset.clone().into())
		}
	}

	impl<T: Config> Pallet<T> {
		/// Generates a new random id for withdrawals
		fn new_random_id() -> Vec<u8> {
			let mut nonce = <RandomnessNonce<T>>::get();
			nonce = nonce.wrapping_add(1);
			<RandomnessNonce<T>>::put(nonce);
			let network_id = T::SubstrateNetworkId::get();
			let entropy = sp_io::hashing::blake2_256(&((network_id, nonce).encode()));
			entropy.to_vec()
		}

		/// Get Pallet Id
		pub fn get_pallet_account() -> T::AccountId {
			T::AssetHandlerPalletId::get().into_account_truncating()
		}

		/// Route deposit to destined function
		pub fn handle_deposit(
			withdrawal: Withdraw,
			location: VersionedMultiLocation,
		) -> DispatchResult {
			let destination_account = Self::get_destination_account(
				location.try_into().map_err(|_| Error::<T>::UnableToConvertToMultiLocation)?,
			)
			.ok_or(Error::<T>::UnableToConvertToAccount)?;
			let pallet_account: T::AccountId =
				T::AssetHandlerPalletId::get().into_account_truncating();
			Self::resolver_deposit(
				withdrawal.asset_id.into(),
				withdrawal.amount,
				&destination_account,
				pallet_account.clone(),
				1u128,
				pallet_account,
			)?;
			Ok(())
		}

		/// Converts Multi-Location to AccountId
		pub fn get_destination_account(location: MultiLocation) -> Option<T::AccountId> {
			match location {
				MultiLocation { parents: 0, interior } => {
					if let Junctions::X1(Junction::AccountId32 { network: _, id }) = interior {
						if let Ok(account) = T::AccountId::decode(&mut &id[..]) {
							Some(account)
						} else {
							None
						}
					} else {
						None
					}
				},
				_ => None,
			}
		}

		/// Check if location is meant for Native Parachain
		pub fn is_polkadex_parachain_destination(destination: &VersionedMultiLocation) -> bool {
			let destination: Option<MultiLocation> = destination.clone().try_into().ok();
			if let Some(destination) = destination {
				destination.parents == 0
			} else {
				false
			}
		}

		/// Checks if asset is meant for Parachain
		pub fn is_parachain_asset(versioned_asset: &VersionedMultiAssets) -> bool {
			let native_asset = MultiLocation { parents: 0, interior: Junctions::Here };
			let assets: Option<MultiAssets> = versioned_asset.clone().try_into().ok();
			if let Some(assets) = assets {
				if let Some(asset) = assets.get(0) {
					matches!(asset.id, AssetId::Concrete(location) if location == native_asset)
				} else {
					false
				}
			} else {
				false
			}
		}

		/// Retrieves the existing assetid for given assetid or generates and stores a new assetid
		pub fn generate_asset_id_for_parachain(asset: AssetId) -> u128 {
			// Check if its native or not.
			if asset
				== AssetId::Concrete(MultiLocation {
					parents: 1,
					interior: Junctions::X1(Parachain(T::ParachainId::get())),
				}) {
				return T::NativeAssetId::get().into();
			}
			// If it's not native, then hash and generate the asset id
			let asset_id = u128::from_be_bytes(sp_io::hashing::blake2_128(&asset.encode()[..]));
			if !<ParachainAssets<T>>::contains_key(asset_id) {
				// Store the mapping
				<ParachainAssets<T>>::insert(asset_id, asset);
			}
			asset_id
		}

		/// Converts XCM::Fungibility into u128
		pub fn get_amount(fun: &Fungibility) -> Option<u128> {
			if let Fungibility::Fungible(amount) = fun {
				Some(*amount)
			} else {
				None
			}
		}

		/// Block Transaction to be Executed.
		pub fn block_by_ele(block_no: BlockNumberFor<T>, index: u32) -> DispatchResult {
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(block_no);
			let pending_withdrawal: &mut Withdraw =
				pending_withdrawals.get_mut(index as usize).ok_or(Error::<T>::IndexNotFound)?;
			pending_withdrawal.is_blocked = true;
			<PendingWithdrawals<T>>::insert(block_no, pending_withdrawals);
			Ok(())
		}

		/// Converts asset_id to XCM::MultiLocation
		pub fn convert_asset_id_to_location(asset_id: u128) -> Option<MultiLocation> {
			Self::assets_mapping(asset_id).and_then(|asset| match asset {
				AssetId::Concrete(location) => Some(location),
				AssetId::Abstract(_) => None,
			})
		}

		/// Converts Multilocation to u128
		pub fn convert_location_to_asset_id(location: MultiLocation) -> u128 {
			Self::generate_asset_id_for_parachain(AssetId::Concrete(location))
		}

		pub fn insert_pending_withdrawal(block_no: BlockNumberFor<T>, withdrawal: Withdraw) {
			<PendingWithdrawals<T>>::insert(block_no, vec![withdrawal]);
		}
	}

	impl<T: Config> AssetIdConverter for Pallet<T> {
		fn convert_asset_id_to_location(asset_id: u128) -> Option<MultiLocation> {
			Self::convert_asset_id_to_location(asset_id)
		}

		fn convert_location_to_asset_id(location: MultiLocation) -> Option<u128> {
			Some(Self::convert_location_to_asset_id(location))
		}
	}

	impl<T: Config> WhitelistedTokenHandler for Pallet<T> {
		fn check_whitelisted_token(asset_id: u128) -> bool {
			let whitelisted_tokens = <WhitelistedTokens<T>>::get();
			whitelisted_tokens.contains(&asset_id)
		}
	}

	impl<T: Config> TheaIncomingExecutor for Pallet<T> {
		fn execute_deposits(_: Network, deposits: Vec<u8>) {
			let deposits = Vec::<Withdraw>::decode(&mut &deposits[..]).unwrap_or_default();
			for deposit in deposits {
				// Calculate the withdrawal execution delay
				let withdrawal_execution_block: BlockNumberFor<T> =
					<frame_system::Pallet<T>>::block_number()
						.saturated_into::<u32>()
						.saturating_add(
							T::WithdrawalExecutionBlockDiff::get().saturated_into::<u32>(),
						)
						.into();
				// Queue the withdrawal for execution
				<PendingWithdrawals<T>>::mutate(
					withdrawal_execution_block,
					|pending_withdrawals| {
						pending_withdrawals.push(deposit);
					},
				);
			}
		}
	}

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
