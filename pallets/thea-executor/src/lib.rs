#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::SaturatedConversion,
		traits::{Currency, ExistenceRequirement, ReservableCurrency},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::AccountIdConversion;
	use std::collections::BTreeSet;
	use thea_primitives::{
		parachain::{
			ApprovedWithdraw, AssetType, ParachainAsset, ParachainDeposit, ParachainWithdraw,
		},
		Network, TheaIncomingExecutor, TheaOutgoingExecutor,
	};
	use xcm::{
		latest::{AssetId, Junction, Junctions, MultiAsset, MultiLocation},
		prelude::{Fungible, X1},
	};

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

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + asset_handler::pallet::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Balances Pallet
		type Currency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;
		/// Asset Create/ Update Origin
		type AssetCreateUpdateOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
		/// Something that executes the payload
		type Executor: thea_primitives::TheaOutgoingExecutor;
		/// Thea PalletId
		#[pallet::constant]
		type TheaPalletId: Get<PalletId>;
		/// Total Withdrawals
		#[pallet::constant]
		type WithdrawalSize: Get<u32>;
		/// Para Id
		type ParaId: Get<u32>;
	}

	/// Withdrawal nonces for each network
	#[pallet::storage]
	#[pallet::getter(fn withdrawal_nonces)]
	pub(super) type WithdrawalNonces<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	/// Withdrawal nonces for each network
	#[pallet::storage]
	#[pallet::getter(fn last_processed_withdrawal_nonce)]
	pub(super) type LastProcessedWithdrawalNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

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
		T::BlockNumber, //Block No
		Blake2_128Concat,
		(u8, u32), //(NetworkId, Withdrawal Nonce)
		(u8, BoundedVec<ApprovedWithdraw, ConstU32<10>>),
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn accounts_with_pending_deposits)]
	pub(super) type AccountWithPendingDeposits<T: Config> =
		StorageValue<_, BTreeSet<T::AccountId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn get_approved_deposits)]
	pub(super) type ApprovedDeposits<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ApprovedDeposit<T::AccountId>, ConstU32<100>>,
		OptionQuery,
	>;

	/// Deposit Nonce for Thea Deposits
	#[pallet::storage]
	#[pallet::getter(fn get_deposit_nonce)]
	pub(super) type DepositNonce<T: Config> =
		StorageMap<_, Blake2_128Concat, Network, u32, ValueQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
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
		/// Deposit Nonce Error
		DepositNonceError,
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
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_initialize(block_no: T::BlockNumber) -> Weight {
			let pending_withdrawals = <ReadyWithdrawls<T>>::iter_prefix_values(block_no);
			for (network_id, withdrawal) in pending_withdrawals {
				T::Executor::execute_withdrawals(network_id, withdrawal.encode());
			}
			//TODO: Clean Storage
			Weight::default()
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn withdraw(
			origin: OriginFor<T>,
			asset_id: u128,
			amount: u128,
			beneficiary: Vec<u8>,
			pay_for_remaining: bool,
		) -> DispatchResult {
			let user = ensure_signed(origin)?;
			Self::do_withdraw(user, asset_id, amount, beneficiary, pay_for_remaining)?;
			Ok(())
		}
	}

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
			ensure!(beneficiary.len() <= 1000, Error::<T>::BeneficiaryTooLong);
			let network = if asset_id == T::NativeCurrencyId::get() {
				1
			} else {
				let (network, ..) = asset_handler::pallet::Pallet::<T>::get_thea_assets(asset_id);
				network
			};
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

			if pending_withdrawals.try_push(withdrawal).is_err() {
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
					<frame_system::Pallet<T>>::block_number(), //Block No
					(network, withdrawal_nonce),
					(network, pending_withdrawals.clone()),
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
			let asset_identifier = if asset_id != T::NativeCurrencyId::get() {
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
			let recipient: MultiLocation =
				Decode::decode(&mut &recipient[..]).map_err(|_| Error::<T>::FailedToDecode)?;
			Ok(recipient)
		}

		pub fn do_deposit(network: Network, payload: Vec<u8>) -> Result<(), DispatchError> {
			let approved_deposit = Self::router(network, payload.clone())?;
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
			Ok(())
		}

		pub fn router(
			network_id: Network,
			payload: Vec<u8>,
		) -> Result<ApprovedDeposit<T::AccountId>, DispatchError> {
			match network_id {
				1 => Self::handle_parachain_deposit(payload),
				2 => unimplemented!(),
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
	}

	impl<T: Config> TheaIncomingExecutor for Pallet<T> {
		fn execute_deposits(network: Network, deposits: Vec<u8>) -> DispatchResult {
			Self::do_deposit(network, deposits)
		}
	}
}
