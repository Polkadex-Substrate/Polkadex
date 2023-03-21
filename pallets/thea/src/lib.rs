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
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unused_unit)]
#![deny(unused_crate_dependencies)]

// TODO[#614]: Thea Pallet Tests
// #[cfg(test)]
// mod mock;
//
// #[cfg(test)]
// mod tests;

#[frame_support::pallet]
pub mod pallet {
	use sp_std::{
		collections::{btree_map::BTreeMap, btree_set::BTreeSet},
		vec::Vec,
	};

	use frame_support::{
		dispatch::fmt::Debug,
		pallet_prelude::*,
		traits::{Currency, ExistenceRequirement, ReservableCurrency},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::{
		traits::{AccountIdConversion, IdentifyAccount, Zero},
		SaturatedConversion,
	};

	use thea_primitives::{
		normal_deposit::Deposit,
		parachain_primitives::{AssetType, ParachainAsset, ParachainDeposit, ParachainWithdraw},
		thea_types::OnSessionChange,
		ApprovedWithdraw, AssetIdConverter, BLSPublicKey, TheaExtrinsicSubmitted,
		TheaPalletMessages, TokenType,
	};
	use thea_staking::SessionChanged;
	use xcm::{
		latest::{AssetId, Junction, Junctions, MultiAsset, MultiLocation, NetworkId},
		prelude::{Fungible, X1},
	};

	use core::default::Default;
	use sp_runtime::traits::Verify;

	pub type Network = u8;

	#[derive(Encode, Decode, Clone, Copy, Debug, MaxEncodedLen, TypeInfo)]
	pub struct ApprovedDeposit<AccountId> {
		pub asset_id: u128,
		pub amount: u128,
		pub recipient: AccountId,
		pub network_id: u8,
		pub tx_hash: sp_core::H256,
		pub deposit_nonce: u32,
	}

	impl<AccountId> ApprovedDeposit<AccountId> {
		fn new(
			asset_id: u128,
			amount: u128,
			recipient: AccountId,
			network_id: u8,
			transaction_hash: sp_core::H256,
			deposit_nonce: u32,
		) -> Self {
			ApprovedDeposit {
				asset_id,
				amount,
				recipient,
				network_id,
				tx_hash: transaction_hash,
				deposit_nonce,
			}
		}
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config:
		frame_system::Config + asset_handler::pallet::Config + thea_staking::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::Origin>;
		/// Thea PalletId
		#[pallet::constant]
		type TheaPalletId: Get<PalletId>;
		/// Total Withdrawals
		#[pallet::constant]
		type WithdrawalSize: Get<u32>;
		/// Para Id
		type ParaId: Get<u32>;
		/// Extrinsic Notifier for rewards
		type ExtrinsicSubmittedNotifier: TheaExtrinsicSubmitted<Self::AccountId>;
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

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Active Relayers BLS Keys for a given Network
	#[pallet::storage]
	#[pallet::getter(fn get_key_rotation_status)]
	pub(super) type TheaKeyRotation<T: Config> =
		StorageMap<_, frame_support::Blake2_128Concat, u8, bool, ValueQuery>;

	/// Active Relayers BLS Keys for a given Network
	#[pallet::storage]
	#[pallet::getter(fn get_relayers_key_vector)]
	pub(super) type RelayersBLSKeyVector<T: Config> =
		StorageMap<_, frame_support::Blake2_128Concat, u8, Vec<BLSPublicKey>, ValueQuery>;

	/// Active Relayers AccountId for a given Network
	#[pallet::storage]
	#[pallet::getter(fn get_auth_list)]
	pub(super) type AuthorityListVector<T: Config> =
		StorageMap<_, frame_support::Blake2_128Concat, u8, Vec<T::AccountId>, ValueQuery>;

	/// Queued Relayers BLS Keys for a given Network ( these are relayers who are waiting for
	/// public key update ack from foreign chain to become active )
	#[pallet::storage]
	#[pallet::getter(fn get_queued_relayers_key_vector)]
	pub(super) type QueuedRelayersBLSKeyVector<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, Vec<BLSPublicKey>, ValueQuery>;

	/// Queued Relayers AccountIds for a given Network ( these are relayers who are waiting for
	// 	/// public key update ack from foreign chain to become active )
	#[pallet::storage]
	#[pallet::getter(fn get_queued_authority_list)]
	pub(super) type QueuedAuthorityListVector<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, Vec<T::AccountId>, ValueQuery>;

	/// Approved Deposits
	#[pallet::storage]
	#[pallet::getter(fn get_approved_deposits)]
	pub(super) type ApprovedDeposits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ApprovedDeposit<T::AccountId>, ConstU32<100>>,
		OptionQuery,
	>;

	/// Pending Withdrawals for batch completion
	#[pallet::storage]
	#[pallet::getter(fn pending_withdrawals)]
	pub(super) type PendingWithdrawals<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		Network,
		BoundedVec<ApprovedWithdraw, ConstU32<10>>,
		ValueQuery,
	>;

	/// Withdrawal Fees for each network
	#[pallet::storage]
	#[pallet::getter(fn witdrawal_fees)]
	pub(super) type WithdrawalFees<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u128, OptionQuery>;

	/// Withdrawal batches ready for sigining
	#[pallet::storage]
	#[pallet::getter(fn ready_withdrawals)]
	pub(super) type ReadyWithdrawls<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		u8,
		Blake2_128Concat,
		u32,
		BoundedVec<ApprovedWithdraw, ConstU32<10>>,
		ValueQuery,
	>;

	/// Withdrawal nonces for each network
	#[pallet::storage]
	#[pallet::getter(fn withdrawal_nonces)]
	pub(super) type WithdrawalNonces<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	/// Accounts which have a pending deposit
	#[pallet::storage]
	#[pallet::getter(fn accounts_with_pending_deposits)]
	pub(super) type AccountWithPendingDeposits<T: Config> =
		StorageValue<_, BTreeSet<T::AccountId>, ValueQuery>;

	/// Asset id to network mapping
	/// u128 => u8
	#[pallet::storage]
	#[pallet::getter(fn asset_id_to_network)]
	pub(super) type AssetIdToNetworkMapping<T: Config> =
		StorageMap<_, Blake2_128Concat, u128, Network, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn active_networks)]
	/// Currently active networks ( this is controlled by thea-staking pallet through the trait
	/// below)
	pub(super) type ActiveNetworks<T: Config> = StorageValue<_, BTreeSet<Network>, ValueQuery>;

	/// Deposit Nonce for Thea Deposits
	#[pallet::storage]
	#[pallet::getter(fn get_deposit_nonce)]
	pub(super) type DepositNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	/// Thea Session Ids for each network
	#[pallet::storage]
	#[pallet::getter(fn get_thea_session_id)]
	pub(super) type TheaSessionId<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	/// Pre-generated Thea public keys for each network for queued relayers from staking pallet
	#[pallet::storage]
	#[pallet::getter(fn get_queued_queued_thea_public_keys)]
	pub(super) type QueuedQueuedTheaPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, [u8; 64], OptionQuery>;

	/// Pre-generated Thea public keys for each network waiting for ack
	#[pallet::storage]
	#[pallet::getter(fn get_queued_thea_public_keys)]
	pub(super) type QueuedTheaPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, [u8; 64], OptionQuery>;

	/// Active Thea public keys for each network
	#[pallet::storage]
	#[pallet::getter(fn get_thea_public_keys)]
	pub(super) type TheaPublicKey<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, [u8; 64], OptionQuery>;

	/// Foreign Chain Ack transactions map
	#[pallet::storage]
	#[pallet::getter(fn foreign_chain_ack_txn)]
	pub(super) type ForeignChainAckTxns<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		Network,
		Blake2_128Concat,
		sp_core::H256,
		u128,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn get_ingress_messages)]
	pub(super) type IngressMessages<T: Config> =
		StorageValue<_, Vec<TheaPalletMessages>, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://substrate.dev/docs/en/knowledgebase/runtime/events
	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Deposit Approved event ( recipient, asset_id, amount, tx_hash(foreign chain))
		DepositApproved(u8, T::AccountId, u128, u128, sp_core::H256),
		/// Deposit claimed event ( recipient, number of deposits claimed )
		DepositClaimed(T::AccountId, u128, u128, sp_core::H256),
		/// Withdrawal Queued ( network, from, beneficiary, assetId, amount, nonce, index )
		WithdrawalQueued(Network, T::AccountId, Vec<u8>, u128, u128, u32, u32),
		/// Withdrawal Ready (Network id, Nonce )
		WithdrawalReady(Network, u32),
		/// Withdrawal Executed (Nonce, network, Tx hash )
		WithdrawalExecuted(u32, Network, sp_core::H256),
		// Thea Public Key Updated ( network, new session id )
		TheaKeyUpdated(Network, u32),
		/// Withdrawal Fee Set (NetworkId, Amount)
		WithdrawalFeeSet(u8, u128),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		// Unable to find Queued Thea Public Key
		QueuedTheaPublicKeyNotFound,
		// Unable to find Queued Queued Thea Public Key
		QueuedQueuedTheaPublicNotFound,
		// Duplicate Transaction Hash
		DuplicateAckTxHash,
		// Nonce does not match
		DepositNonceError,
		/// Amount cannot be zero
		AmountCannotBeZero,
		/// Asset has not been registered
		AssetNotRegistered,
		/// BLS Aggregate signature failed
		BLSSignatureVerificationFailed,
		/// Beneficiary Size too long
		BeneficiaryTooLong,
		/// Unable to find mapping between asset id to network
		UnableFindNetworkForAssetId,
		/// Too many withdrawals in queue,
		WithdrawalNotAllowed,
		/// Withdrawal fee is not configured this network
		WithdrawalFeeConfigNotFound,
		/// No approved deposits for the provided account
		NoApprovedDeposit,
		/// Token type not handled
		TokenTypeNotHandled,
		/// Failed To Decode
		FailedToDecode,
		/// Failed To Handle Parachain Deposit
		FailedToHandleParachainDeposit,
		/// Failed to get AssetId
		FailedToGetAssetId,
		/// Bounded Vector Overflow
		BoundedVectorOverflow,
		/// Bounded Vector Not Present
		BoundedVectorNotPresent,
		/// Thea Key Rotation is taking place
		TheaKeyRotationInPlace,
		/// Relayer is absent in active set
		RelayerAbsentInActiveSet,
	}

	// Hooks for Thea Pallet are defined here
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		// fn on_idle(_n: BlockNumberFor<T>, mut remaining_weight: Weight) -> Weight {
		// 	// TODO: Calculate proper weight for single claim call on on_idle
		// 	let single_claim_weight: Weight = 100_000_000;
		//
		// 	if remaining_weight < single_claim_weight {
		// 		// We need enough weight for at least one claim process if not it's a no-op
		// 		return remaining_weight
		// 	}
		//
		// 	let mut accounts = <AccountWithPendingDeposits<T>>::get();
		// 	if accounts.is_empty() {
		// 		return remaining_weight
		// 	}
		//
		// 	while let Some(account) = accounts.pop_first() {
		// 		if let Some(mut pending_deposits) = <ApprovedDeposits<T>>::get(&account) {
		// 			// FIXME: This leads to an infinite loop if execute_deposit fails
		// 			while let Some(deposit) = pending_deposits.pop() {
		// 				if let Err(err) = Self::execute_deposit(deposit.clone(), &account) {
		// 					// Force push is fine as it was part of the bounded vec
		// 					pending_deposits.force_push(deposit.clone());
		// 					// We can't do much here other than to log an error.
		// 					log::error!(target:"runtime::thea::on_idle","Error while claiming deposit on idle: user:
		// {:?}, Err: {:?}",account,err); 				}
		// 				// reduce the remaining_weight
		// 				remaining_weight = remaining_weight.saturating_sub(single_claim_weight);
		// 				if remaining_weight.is_zero() {
		// 					break
		// 				}
		// 			}
		//
		// 			if !pending_deposits.is_empty() {
		// 				<ApprovedDeposits<T>>::insert(&account, pending_deposits);
		// 				accounts.insert(account);
		// 			}
		// 		}
		// 	}
		// 	<AccountWithPendingDeposits<T>>::put(accounts);
		// 	remaining_weight
		// }
		//
		fn on_initialize(_n: BlockNumberFor<T>) -> Weight {
			<IngressMessages<T>>::put(Vec::<TheaPalletMessages>::new());
			// TODO: Benchmarking for Thea Pallet
			1000 as Weight
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;
		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			// A closure that takes a `&Network` and a `&u8` relayer index as inputs and returns an
			// optional AccountId.
			let get_account_id = |network: &Network, relayer_index: &u8| {
				let relayer_account_vector = <AuthorityListVector<T>>::get(network);
				let relayer_account = relayer_account_vector.get(*relayer_index as usize);
				if let Some(relayer_account) = relayer_account {
					return Some(relayer_account.clone())
				}
				None
			};

			// Verify provided payload by provided signature
			let verify_signature =
				|signature: &T::Signature, payload: &[u8], signer: T::AccountId| {
					signature.verify(payload, &signer)
				};

			// A closure that takes a byte array reference `tx_tag` as input and returns a
			// `ValidTransaction`.
			let generate_valid_tx = |tx_tag: &[u8; 30]| {
				ValidTransaction::with_tag_prefix("thea-proc")
					.priority(T::UnsignedPriority::get())
					.and_provides([&(tx_tag)])
					.longevity(3)
					.propagate(true)
					.build()
			};

			// A closure that takes a byte array reference `payload`, a reference to a `u8` relayer
			// index, a `&Network` instance representing the network in which the relayer is
			// registered, a `&T::Signature` signature, and a byte array reference `tx_tag` as input
			// and returns either a `ValidTransaction` or `InvalidTransaction`.
			let generate_tx = |payload: &[u8],
			                   relayer_index: &u8,
			                   network: &Network,
			                   signature: &T::Signature,
			                   tx_tag: &[u8; 30]| {
				match get_account_id(network, relayer_index) {
					Some(relayer_account) => {
						if verify_signature(signature, payload, relayer_account) {
							generate_valid_tx(tx_tag)
						} else {
							InvalidTransaction::Call.into()
						}
					},
					None => InvalidTransaction::Call.into(),
				}
			};

			match call {
				Call::approve_deposit {
					bit_map: _,
					bls_signature: _,
					token_type: _,
					payload,
					relayer_index,
					network,
					signature,
				} =>
					return generate_tx(
						payload.as_slice(),
						relayer_index,
						network,
						signature,
						b"thea_unsigned_approved_depsoit",
					),
				Call::set_thea_key_complete {
					network,
					public_key,
					bit_map: _,
					bls_signature: _,
					relayer_index,
					signature,
				} =>
					return generate_tx(
						public_key.as_slice(),
						relayer_index,
						network,
						signature,
						b"thea_unsigned_key_completed___",
					),
				Call::thea_queued_queued_public_key {
					network,
					public_key,
					bit_map: _,
					bls_signature: _,
					relayer_index,
					signature,
				} =>
					return generate_tx(
						public_key.as_slice(),
						relayer_index,
						network,
						signature,
						b"thea_unsigned_qq_key_completed",
					),
				Call::thea_key_rotation_complete {
					network,
					tx_hash,
					bit_map: _,
					bls_signature: _,
					relayer_index,
					signature,
				} =>
					return generate_tx(
						tx_hash.as_bytes(),
						relayer_index,
						network,
						signature,
						b"thea_key_rotation_completed___",
					),
				_ => InvalidTransaction::Call.into(),
			}
		}
	}

	// Extrinsic for Thea Pallet are defined here
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		///Approve Deposit
		///
		/// # Parameters
		///
		/// * `bit_map`: Relayers signed the payload.
		/// * `bls_signature`: BLS Signature.
		/// * `token_type`: Token Type.
		/// * `payload`: Encoded Deposit Payload.
		/// * `_relayer_index`: The index of relayer in active set will be used to validate unsigned
		///   tx.
		/// * `_network`: Network id will be used to validate unsigned tx.
		/// * `_signature`: The signature of the relayer on the payload will be used to validate
		///   unsigned tx.
		#[pallet::weight(1000)]
		pub fn approve_deposit(
			origin: OriginFor<T>,
			bit_map: u128,
			bls_signature: [u8; 96],
			token_type: TokenType,
			payload: Vec<u8>,
			_relayer_index: u8,
			_network: Network,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			Self::do_deposit(token_type, payload, bit_map, bls_signature)?;
			Ok(Pays::No.into())
		}

		/// Manually claim an approved deposit
		///
		/// # Parameters
		///
		/// * `origin`: User
		/// * `num_deposits`: Number of deposits to claim from available deposits,
		/// (it's used to parametrise the weight of this extrinsic)
		// TODO: [Issue #606] Use benchmarks
		#[pallet::weight(1000)]
		pub fn claim_deposit(origin: OriginFor<T>, num_deposits: u32) -> DispatchResult {
			let user = ensure_signed(origin)?;

			if let Some(mut deposits) = <ApprovedDeposits<T>>::get(&user) {
				let length: u32 = if deposits.len().saturated_into::<u32>() <= num_deposits {
					deposits.len().saturated_into()
				} else {
					num_deposits
				}
				.saturated_into();

				for _ in 0..length {
					if let Some(deposit) = deposits.pop() {
						if let Err(err) = Self::execute_deposit(deposit.clone(), &user) {
							// Force push is fine as it will have the capacity.
							deposits.force_push(deposit);
							// Save it back on failure
							<ApprovedDeposits<T>>::insert(&user, deposits.clone());
							return Err(err)
						}
					} else {
						break
					}
				}

				if !deposits.is_empty() {
					// If pending deposits are available, save it back
					<ApprovedDeposits<T>>::insert(&user, deposits)
				} else {
					<AccountWithPendingDeposits<T>>::mutate(|accounts| accounts.remove(&user));
				}
			} else {
				return Err(Error::<T>::NoApprovedDeposit.into())
			}

			Ok(())
		}

		/// Extrinsic to update withdrawal completion status by relayer
		///
		/// # Parameters
		///
		/// * `origin`: Any relayer
		/// * `withdrawal_nonce`: Withdrawal Nonce
		/// * `network`: Network id
		/// * `tx_hash`: Vec<u8>
		/// * `bit_map`: Bitmap of Thea relayers
		/// * `bls_signature`: BLS signature of relayers
		// TODO: [Issue #606] Use benchmarks
		#[pallet::weight(1000)]
		pub fn batch_withdrawal_complete(
			origin: OriginFor<T>,
			withdrawal_nonce: u32,
			network: Network,
			tx_hash: sp_core::H256,
			_bit_map: u128,
			_bls_signature: [u8; 96],
		) -> DispatchResult {
			ensure_signed(origin)?;

			// TODO: This will be refactored when work on withdrawal begins
			<ReadyWithdrawls<T>>::take(network, withdrawal_nonce);
			Self::deposit_event(Event::<T>::WithdrawalExecuted(withdrawal_nonce, network, tx_hash));
			Ok(())
		}

		/// Initiate Withdrawal request
		///
		/// # Parameters
		///
		/// * `origin`: User
		/// * `asset_id`: Asset id
		/// * `amount`: Amount of asset to withdraw
		/// * `beneficiary`: beneficiary of the withdraw
		/// * `pay_for_remaining`: user is ready to pay for remaining pending withdrawal for quick
		///   withdrawal
		// TODO: [Issue #606] Use benchmarks
		#[pallet::weight(1000)]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			// Put a soft limit of size of beneficiary vector to avoid spam
			ensure!(beneficiary.len() <= 100, Error::<T>::BeneficiaryTooLong);
			Self::do_withdraw(user, asset_id, amount, beneficiary, pay_for_remaining)?;
			Ok(())
		}

		/// Add Token Config
		///
		/// # Parameters
		///
		/// * `network_id`: Network Id.
		/// * `fee`: Withdrawal Fee.
		#[pallet::weight(1000)]
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

		/// Extrinsic to acknowledge on chain state key change completion on all foreign chains
		///
		/// # Parameters
		///
		/// * `origin`: Any relayer
		/// * `network`: Network id
		/// * `tx_hash`: Transaction hash of key update on foreign chain
		/// * `bit_map`: Bitmap of Thea relayers
		/// * `bls_signature`: BLS signature of relayers
		/// * `relayer_index`: The index of relayer in active set.
		/// * `_signature`: The signature of the relayer on tx_hash will be used to validate
		///   unsigned tx.
		// TODO: [Issue #606] Use benchmarks
		#[pallet::weight(1000)]
		pub fn thea_key_rotation_complete(
			origin: OriginFor<T>,
			network: Network,
			tx_hash: sp_core::H256,
			bit_map: u128,
			bls_signature: [u8; 96],
			relayer_index: u8,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let relayer = Self::get_relayer_account_id(&network, &relayer_index)?;

			// Check if tx_hash is already included
			ensure!(
				<ForeignChainAckTxns<T>>::get(network, tx_hash).is_none(),
				Error::<T>::DuplicateAckTxHash
			);

			// Fetch current active relayer set BLS Keys
			let current_relayer_set = Self::get_relayers_key_vector(network);
			let current_relayer_set_accounts = <AuthorityListVector<T>>::get(network);

			// Call host function with current_active_relayer_set, signature, bit_map, verify nonce
			ensure!(
				thea_primitives::thea_ext::bls_verify(
					&bls_signature,
					bit_map,
					&(tx_hash, network).encode(),
					&current_relayer_set
				),
				Error::<T>::BLSSignatureVerificationFailed
			);

			Self::move_queued_to_active(network)?;
			// To avoid replay attack
			<ForeignChainAckTxns<T>>::insert(network, tx_hash, bit_map);
			// Emit an ingress message
			<IngressMessages<T>>::mutate(|messages| {
				messages.push(TheaPalletMessages::TheaKeyRotationComplete)
			});

			<TheaKeyRotation<T>>::insert(network, false);
			T::ExtrinsicSubmittedNotifier::thea_extrinsic_submitted(
				relayer,
				bit_map,
				current_relayer_set_accounts,
			);

			Ok(Pays::No.into())
		}

		/// Extrinsic to update solo chain that a new Thea Key has been set by Sudo
		///
		/// # Parameters
		///
		/// * `origin`: Any relayer
		/// * `network`: Network id
		/// * `public_key`: New Public Key for thea (Raw Uncompressed)
		/// * `bit_map`: Bitmap of Thea relayers
		/// * `bls_signature`: BLS signature of relayers
		/// * `relayer_index`: The index of relayer in active set.
		/// * `_signature`: The signature of the relayer on public_key will be used to validate
		///   unsigned tx.
		// TODO: [Issue #606] Use benchmarks
		#[pallet::weight(1000)]
		pub fn set_thea_key_complete(
			origin: OriginFor<T>,
			network: Network,
			public_key: [u8; 64],
			bit_map: u128,
			bls_signature: [u8; 96],
			relayer_index: u8,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			let relayer = Self::get_relayer_account_id(&network, &relayer_index)?;
			// Verify BLS Signature
			// Fetch Current BLS Keys
			let current_thea_key = <TheaPublicKey<T>>::get(network).unwrap_or([0_u8; 64]);
			ensure!(public_key != current_thea_key, Error::<T>::QueuedTheaPublicKeyNotFound);
			let bls_keys = Self::get_relayers_key_vector(network);
			let authority_set = <AuthorityListVector<T>>::get(network);
			// Call Host Function
			ensure!(
				thea_primitives::thea_ext::bls_verify(
					&bls_signature,
					bit_map,
					&public_key.encode(),
					&bls_keys
				),
				Error::<T>::BLSSignatureVerificationFailed
			);
			// Update Active Public Key
			<TheaPublicKey<T>>::insert(network, public_key);
			// Incrementing Current Round Index
			let current_round_index = <TheaSessionId<T>>::get(network);
			<TheaSessionId<T>>::insert(network, current_round_index.saturating_add(1));
			<TheaKeyRotation<T>>::insert(network, false);
			Self::deposit_event(Event::TheaKeyUpdated(network, current_round_index - 1));
			T::ExtrinsicSubmittedNotifier::thea_extrinsic_submitted(
				relayer,
				bit_map,
				authority_set,
			);
			Ok(Pays::No.into())
		}

		/// Extrinsic to acknowledge on chain state key change completion on all foreign chains
		///
		/// # Parameters
		///
		/// * `origin`: Any relayer
		/// * `network`: Network id
		/// * `public_key`: Thea Public Key
		/// * `bit_map`: Bitmap of Thea relayers
		/// * `bls_signature`: BLS signature of relayers
		/// * `_relayer_index`: The index of relayer in active set will be used to validate unsigned
		///   tx.
		/// * `_signature`: The signature of the relayer on public_key will be used to validate
		///   unsigned tx.
		// TODO: [Issue #606] Use benchmarks
		#[pallet::weight(1000)]
		pub fn thea_queued_queued_public_key(
			origin: OriginFor<T>,
			network: Network,
			public_key: [u8; 64],
			bit_map: u128,
			bls_signature: [u8; 96],
			_relayer_index: u8,
			_signature: T::Signature,
		) -> DispatchResultWithPostInfo {
			ensure_none(origin)?;
			// Fetch current active relayer set BLS Keys

			let current_public_key =
				<QueuedQueuedTheaPublicKey<T>>::get(network).unwrap_or([0_u8; 64]);
			ensure!(public_key != current_public_key, Error::<T>::QueuedTheaPublicKeyNotFound);

			let queued_queued_relayers =
				thea_staking::Pallet::<T>::get_queued_relayers_bls_keys(network);

			// Call host function with current_active_relayer_set, signature, bit_map, verify nonce
			ensure!(
				thea_primitives::thea_ext::bls_verify(
					&bls_signature,
					bit_map,
					&public_key.encode(),
					&queued_queued_relayers
				),
				Error::<T>::BLSSignatureVerificationFailed
			);

			// Move queued_queued to queued
			if let Some(queued_queued) = <QueuedQueuedTheaPublicKey<T>>::take(network) {
				<QueuedTheaPublicKey<T>>::insert(network, queued_queued);
				<QueuedQueuedTheaPublicKey<T>>::insert(network, public_key);
				// Emit an Ingress Message to sign the Qd Public Key
				<IngressMessages<T>>::mutate(|messages| {
					messages.push(TheaPalletMessages::SignQdPublicKey)
				});
				<TheaKeyRotation<T>>::insert(network, true);
			} else {
				// If there is no QQPublicKey already then we should set the one we received
				// as the new QQPublicKey rather than returning an Error
				<QueuedQueuedTheaPublicKey<T>>::insert(network, public_key);
			}
			// Add the new one to queued_queued
			Ok(Pays::No.into())
		}

		/// Extrinsic to reset Thea Key Rotation
		///
		/// # Parameters
		///
		/// * `origin`: Any relayer
		/// * `network`: Network id
		#[pallet::weight(1000)]
		pub fn thea_relayers_reset_rotation(
			origin: OriginFor<T>,
			network: Network,
		) -> DispatchResult {
			ensure_root(origin)?;
			<AuthorityListVector<T>>::insert::<u8, Vec<T::AccountId>>(network, Default::default());
			<RelayersBLSKeyVector<T>>::insert::<u8, Vec<BLSPublicKey>>(network, Default::default());
			<QueuedAuthorityListVector<T>>::insert::<u8, Vec<T::AccountId>>(
				network,
				Default::default(),
			);
			<QueuedRelayersBLSKeyVector<T>>::insert::<u8, Vec<BLSPublicKey>>(
				network,
				Default::default(),
			);
			<TheaPublicKey<T>>::take(network);
			<QueuedTheaPublicKey<T>>::take(network);
			<QueuedQueuedTheaPublicKey<T>>::take(network);
			<TheaSessionId<T>>::insert(network, 0);
			Ok(())
		}
	}

	// Helper Functions for Thea Pallet
	impl<T: Config> Pallet<T> {
		pub fn thea_account() -> T::AccountId {
			T::TheaPalletId::get().into_account_truncating()
		}

		pub fn do_withdraw(
			user: T::AccountId,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
		) -> Result<(), DispatchError> {
			ensure!(beneficiary.len() <= 100, Error::<T>::BeneficiaryTooLong);
			let network = if asset_id == T::PolkadexAssetId::get() {
				1
			} else {
				let (network, ..) = asset_handler::pallet::Pallet::<T>::get_thea_assets(asset_id);
				network
			};
			ensure!(network != 0, Error::<T>::UnableFindNetworkForAssetId);
			ensure!(!Self::get_key_rotation_status(network), Error::<T>::TheaKeyRotationInPlace);
			let payload = Self::withdrawal_router(network, asset_id, amount, beneficiary.clone())?;
			let withdrawal_nonce = <WithdrawalNonces<T>>::get(network);
			let mut pending_withdrawals = <PendingWithdrawals<T>>::get(network);
			// Ensure pending withdrawals have space for a new withdrawal
			ensure!(!pending_withdrawals.is_full(), Error::<T>::WithdrawalNotAllowed);

			#[allow(clippy::unnecessary_lazy_evaluations)]
			// TODO: This will be refactored when work on withdrawal so not fixing clippy suggestion
			let mut total_fees = <WithdrawalFees<T>>::get(network)
				.ok_or_else(|| Error::<T>::WithdrawalFeeConfigNotFound)?;

			if pay_for_remaining {
				// User is ready to pay for remaining pending withdrawal for quick withdrawal
				let extra_withdrawals_available =
					T::WithdrawalSize::get().saturating_sub(pending_withdrawals.len() as u32);
				total_fees = total_fees.saturating_add(
					total_fees.saturating_mul(extra_withdrawals_available.saturated_into()),
				)
			}

			// Pay the fees
			<T as Config>::Currency::transfer(
				&user,
				&Self::thea_account(),
				total_fees.saturated_into(),
				ExistenceRequirement::KeepAlive,
			)?;

			// TODO[#610]: Update Thea Staking pallet about fees collected
			// Handle assets
			asset_handler::pallet::Pallet::<T>::handle_asset(asset_id, user.clone(), amount)?;
			let withdrawal = ApprovedWithdraw {
				asset_id,
				amount: amount.saturated_into(),
				network: network.saturated_into(),
				beneficiary: beneficiary.clone(),
				payload,
				index: pending_withdrawals.len() as u32,
			};

			if let Err(()) = pending_withdrawals.try_push(withdrawal) {
				// This should not fail because of is_full check above
			}
			Self::deposit_event(Event::<T>::WithdrawalQueued(
				network,
				user,
				beneficiary,
				asset_id,
				amount,
				withdrawal_nonce,
				(pending_withdrawals.len() - 1) as u32,
			));
			if pending_withdrawals.is_full() | pay_for_remaining {
				// If it is full then we move it to ready queue and update withdrawal nonce
				let withdrawal_nonce = <WithdrawalNonces<T>>::get(network);
				<ReadyWithdrawls<T>>::insert(
					network,
					withdrawal_nonce,
					pending_withdrawals.clone(),
				);
				<WithdrawalNonces<T>>::insert(network, withdrawal_nonce.saturating_add(1));
				Self::deposit_event(Event::<T>::WithdrawalReady(network, withdrawal_nonce));
				pending_withdrawals = BoundedVec::default();
			}
			<PendingWithdrawals<T>>::insert(network, pending_withdrawals);
			Ok(())
		}

		pub fn withdrawal_router(
			network_id: u8,
			asset_id: u128,
			amount: u128,
			recipient: Vec<u8>,
		) -> Result<Vec<u8>, DispatchError> {
			match network_id {
				1 => Self::handle_parachain_withdraw(asset_id, amount, recipient),
				_ => unimplemented!(),
			}
		}

		pub fn handle_parachain_withdraw(
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
		) -> Result<Vec<u8>, DispatchError> {
			let asset_identifier = if asset_id != T::PolkadexAssetId::get() {
				let (_, _, asset_identifier) =
					asset_handler::pallet::TheaAssets::<T>::get(asset_id);
				let asset_identifier: ParachainAsset =
					Decode::decode(&mut &asset_identifier.to_vec()[..])
						.map_err(|_| Error::<T>::FailedToDecode)?;
				asset_identifier
			} else {
				let para_id = T::ParaId::get();
				let asset_location = MultiLocation {
					parents: 1,
					interior: Junctions::X1(Junction::Parachain(para_id)),
				};
				ParachainAsset { location: asset_location, asset_type: AssetType::Fungible }
			};
			let asset_id = AssetId::Concrete(asset_identifier.location);
			let asset_and_amount = MultiAsset { id: asset_id, fun: Fungible(amount) };
			let recipient: MultiLocation = Self::get_recipient(beneficiary)?;
			let parachain_withdraw =
				ParachainWithdraw::get_parachain_withdraw(asset_and_amount, recipient);
			Ok(parachain_withdraw.encode())
		}

		pub fn get_recipient(recipient: Vec<u8>) -> Result<MultiLocation, DispatchError> {
			let recipient: [u8; 32] =
				recipient.try_into().map_err(|_| Error::<T>::DepositNonceError)?; //TODO Handle error
			Ok(MultiLocation {
				parents: 1,
				interior: Junctions::X1(Junction::AccountId32 {
					network: NetworkId::Any,
					id: recipient,
				}),
			})
		}

		pub fn do_deposit(
			token_type: TokenType,
			payload: Vec<u8>,
			bit_map: u128,
			bls_signature: [u8; 96],
		) -> Result<(), DispatchError> {
			let approved_deposit = Self::router(token_type, payload.clone())?;
			let current_active_relayer_set =
				Self::get_relayers_key_vector(approved_deposit.network_id);

			ensure!(
				thea_primitives::thea_ext::bls_verify(
					&bls_signature,
					bit_map,
					&payload,
					&current_active_relayer_set
				),
				Error::<T>::BLSSignatureVerificationFailed
			);

			if <ApprovedDeposits<T>>::contains_key(&approved_deposit.recipient) {
				<ApprovedDeposits<T>>::try_mutate(
					approved_deposit.recipient.clone(),
					|bounded_vec| {
						if let Some(inner_bounded_vec) = bounded_vec {
							inner_bounded_vec
								.try_push(approved_deposit.clone())
								.map_err(|_| Error::<T>::BoundedVectorOverflow)?;
							Ok::<(), Error<T>>(())
						} else {
							Err(Error::<T>::BoundedVectorNotPresent)
						}
					},
				)?;
			} else {
				let mut my_vec: BoundedVec<ApprovedDeposit<T::AccountId>, ConstU32<100>> =
					Default::default();
				if let Ok(()) = my_vec.try_push(approved_deposit.clone()) {
					<ApprovedDeposits<T>>::insert::<
						T::AccountId,
						BoundedVec<ApprovedDeposit<T::AccountId>, ConstU32<100>>,
					>(approved_deposit.recipient.clone(), my_vec);
					<AccountWithPendingDeposits<T>>::mutate(|accounts| {
						accounts.insert(approved_deposit.recipient.clone())
					});
				} else {
					return Err(Error::<T>::BoundedVectorOverflow.into())
				}
			}
			<DepositNonce<T>>::insert(
				approved_deposit.network_id.saturated_into::<Network>(),
				approved_deposit.deposit_nonce,
			);
			Self::deposit_event(Event::<T>::DepositApproved(
				approved_deposit.network_id,
				approved_deposit.recipient,
				approved_deposit.asset_id,
				approved_deposit.amount,
				approved_deposit.tx_hash,
			));
			Ok(())
		}
	}

	impl<T: Config> SessionChanged for Pallet<T> {
		type Network = Network;
		type OnSessionChange = OnSessionChange<T::AccountId>;
		fn on_new_session(map: BTreeMap<Self::Network, Self::OnSessionChange>) {
			//loop through BTreeMap and insert the new BLS pub keys and account ids for each
			// network
			for (network_id, (vec_of_bls_keys, vec_of_account_ids)) in map {
				let current_round = <TheaSessionId<T>>::get(network_id);
				// Check if it is genesis round
				if current_round.is_zero() {
					<RelayersBLSKeyVector<T>>::insert(network_id, vec_of_bls_keys);
					<AuthorityListVector<T>>::insert(network_id, vec_of_account_ids);
				} else {
					<QueuedRelayersBLSKeyVector<T>>::insert(network_id, vec_of_bls_keys);
					<QueuedAuthorityListVector<T>>::insert(network_id, vec_of_account_ids);
				}
				// Inform ingress message to Relayer
				<IngressMessages<T>>::mutate(|messages| {
					messages.push(TheaPalletMessages::EcdsaReady(10))
				});
			}
		}

		// Update the local storage about all networks
		fn set_new_networks(networks: BTreeSet<Network>) {
			<ActiveNetworks<T>>::put(networks)
		}
	}

	// Helper Functions for Thea Pallet
	impl<T: Config> Pallet<T> {
		/// Retrieve the AccountId of the relayer account at the specified index for the specified
		/// network id
		///
		/// Parameters:
		///
		/// `network`: A reference to a Network instance representing the network in which the
		/// relayer account is registered.
		/// `relayer_index`: A reference to a u8 value representing
		/// the index of the relayer account in the list of registered relayers for the specified
		/// network.
		///
		/// Returns:
		/// A `Result::Ok` containing the `AccountId` of the relayer.
		/// A `DispatchError` with the `RelayerAbsentInActiveSet` error code if the relayer account
		/// is not found.
		pub fn get_relayer_account_id(
			network: &Network,
			relayer_index: &u8,
		) -> Result<T::AccountId, DispatchError> {
			let relayer_account_vector = <AuthorityListVector<T>>::get(network);
			let relayer_account = relayer_account_vector.get(*relayer_index as usize);
			if let Some(relayer_account) = relayer_account {
				return Ok(relayer_account.clone())
			}
			Err(Error::<T>::RelayerAbsentInActiveSet.into())
		}

		// Move Queued Authoritys and BLSKeys to Active Storage. It will triggered by
		// submission of new thea key updation on all foreign chains to Polkadex.
		pub fn move_queued_to_active(network: Network) -> DispatchResult {
			let (vec_of_bls_keys, vec_of_account_ids, public_key) = (
				<QueuedRelayersBLSKeyVector<T>>::get(network),
				<QueuedAuthorityListVector<T>>::get(network),
				<QueuedTheaPublicKey<T>>::get(network),
			);
			let public_key = match public_key {
				None => return Err(Error::<T>::QueuedTheaPublicKeyNotFound.into()),
				Some(key) => key,
			};
			<RelayersBLSKeyVector<T>>::insert(network, vec_of_bls_keys);
			<AuthorityListVector<T>>::insert(network, vec_of_account_ids);
			<TheaPublicKey<T>>::insert(network, public_key);
			let current_session_id = <TheaSessionId<T>>::get(network).saturating_add(1);
			<TheaSessionId<T>>::insert(network, current_session_id);
			// TODO: Add IngressMessages::RelayerSetChange(current_session_id) to notify all
			// relayers of relayer set change ( it will be added after #635 is merged to develop )
			Self::deposit_event(Event::<T>::TheaKeyUpdated(network, current_session_id));
			Ok(())
		}

		pub fn router(
			token_type: TokenType,
			payload: Vec<u8>,
		) -> Result<ApprovedDeposit<T::AccountId>, DispatchError> {
			match token_type {
				TokenType::Fungible(network_id) if network_id == 1 =>
					Self::handle_parachain_deposit(payload),
				TokenType::Fungible(network_id) if network_id == 2 =>
					Self::handle_normal_deposit(payload),
				_ => Err(Error::<T>::TokenTypeNotHandled.into()),
			}
		}

		pub fn handle_parachain_deposit(
			payload: Vec<u8>,
		) -> Result<ApprovedDeposit<T::AccountId>, DispatchError> {
			let parachain_deposit: ParachainDeposit =
				Decode::decode(&mut &payload[..]).map_err(|_| Error::<T>::FailedToDecode)?;
			if let (Some(recipient), Some((asset, amount))) = (
				Self::convert_multi_location_to_recipient_address(&parachain_deposit.recipient),
				parachain_deposit.convert_multi_asset_to_asset_id_and_amount(),
			) {
				let network_id: u8 = asset_handler::pallet::Pallet::<T>::get_parachain_network_id();
				Self::validation(parachain_deposit.deposit_nonce, asset, amount, network_id)?;
				Ok(ApprovedDeposit::new(
					asset,
					amount,
					recipient,
					network_id,
					parachain_deposit.transaction_hash,
					parachain_deposit.deposit_nonce,
				))
			} else {
				Err(Error::<T>::FailedToHandleParachainDeposit.into())
			}
		}

		pub fn handle_normal_deposit(
			payload: Vec<u8>,
		) -> Result<ApprovedDeposit<T::AccountId>, DispatchError> {
			let deposit =
				Deposit::decode(&mut &payload[..]).map_err(|_| Error::<T>::FailedToDecode)?;
			let asset_id = deposit.get_asset_id().ok_or(Error::<T>::FailedToGetAssetId)?;
			Self::validation(deposit.deposit_nonce, asset_id, deposit.amount, deposit.network_id)?;
			Ok(ApprovedDeposit::new(
				asset_id,
				deposit.amount,
				deposit.recipient,
				deposit.network_id,
				deposit.transaction_hash,
				deposit.deposit_nonce,
			))
		}

		pub fn convert_multi_location_to_recipient_address(
			recipient_address: &MultiLocation,
		) -> Option<T::AccountId> {
			match recipient_address {
				MultiLocation {
					parents: _,
					interior: X1(Junction::AccountId32 { network: _, id }),
				} => T::AccountId::decode(&mut &id[..]).ok(),
				_ => None,
			}
		}

		pub fn validation(
			deposit_nonce: u32,
			asset_id: u128,
			amount: u128,
			network_id: u8,
		) -> Result<(), DispatchError> {
			ensure!(amount > 0, Error::<T>::AmountCannotBeZero);
			// Fetch Deposit Nonce
			let nonce = <DepositNonce<T>>::get(network_id.saturated_into::<Network>());
			ensure!(deposit_nonce == nonce + 1, Error::<T>::DepositNonceError);
			// Ensure assets are registered
			ensure!(
				asset_handler::pallet::TheaAssets::<T>::contains_key(asset_id),
				Error::<T>::AssetNotRegistered
			);
			Ok(())
		}

		pub fn execute_deposit(
			deposit: ApprovedDeposit<T::AccountId>,
			recipient: &T::AccountId,
		) -> Result<(), DispatchError> {
			asset_handler::pallet::Pallet::<T>::mint_thea_asset(
				deposit.asset_id,
				recipient.clone(),
				deposit.amount,
			)?;
			// Emit event
			Self::deposit_event(Event::<T>::DepositClaimed(
				recipient.clone(),
				deposit.asset_id,
				deposit.amount,
				deposit.tx_hash,
			));
			Ok(())
		}
	}
}
