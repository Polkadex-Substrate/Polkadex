#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]

use crate::pallet::{ Config, Event, Pallet};
use frame_support::dispatch::{DispatchInfo, PostDispatchInfo};
use parity_scale_codec::{Decode, Encode};
use scale_info::{TypeInfo};
use sp_runtime::{
	traits::{
		DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension,
	},
	transaction_validity::{
		TransactionValidity, TransactionValidityError, ValidTransaction,
	},
	DispatchResult,
};


#[frame_support::pallet]
pub mod pallet {
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;
	use sp_runtime::traits::{DispatchInfoOf, PostDispatchInfoOf};

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	/// Configure the pallet by specifying the parameters and types on which it depends.
	pub trait Config: frame_system::Config + pallet_assets::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn allowed_assets)]
	pub type AllowedAssets<T: Config> = StorageValue<_, Vec<T::AssetId>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionFeePaid {
			who: T::AccountId,
			actual_fee: T::Balance,
			tip: T::Balance,
			asset: Option<T::AssetId>,
		},
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Migration is not operational yet
		NotOperational,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {}

	impl<T: Config> Pallet<T> {
		pub fn withdraw_fee(
			who: &T::AccountId,
			call: &T::Call,
			info: &DispatchInfoOf<T::Call>,
			len: usize,
			tip: T::Balance,
			asset: Option<T::AssetId>,
		) -> Result<T::Balance, TransactionValidityError> {
			// TODO: 1) Compute final fee in PDEX
			// 2) Swap alternate currency to PDEX for Fees
			//
			todo!()
		}

		pub fn get_priority(
			info: &DispatchInfoOf<T::Call>,
			len: usize,
			tip: T::Balance,
			final_fee: T::Balance,
		) -> TransactionPriority {
			// TODO: Calculate priority based on fee
			todo!()
		}

		pub fn compute_actual_fee(
			len: usize,
			info: &DispatchInfoOf<T::Call>,
			post_info: &PostDispatchInfoOf<T::Call>,
			tip: T::Balance,
		) -> T::Balance {
			todo!()
		}

		pub fn settle_balance_fee(
			who: &T::AccountId,
			info: &DispatchInfoOf<T::Call>,
			post_info: &PostDispatchInfoOf<T::Call>,
			actual_fee: T::Balance,
			tip: T::Balance,
			imbalance: T::Balance,
		) -> Result<(), TransactionValidityError> {
			todo!()
		}
	}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ChargeTransactionPayment<T: Config>((T::Balance, Option<T::AssetId>));

impl<T: Config> sp_std::fmt::Debug for ChargeTransactionPayment<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "ChargeTransactionPayment<{:?}>", self.0)
	}
	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T: Config> SignedExtension for ChargeTransactionPayment<T>
where
	T::Call: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
{
	const IDENTIFIER: &'static str = "AssetsTransactionPayment";
	type AccountId = T::AccountId;
	type Call = T::Call;
	type AdditionalSigned = Option<T::AssetId>;
	type Pre = (
		// Fee paid in asset
		Option<T::AssetId>,
		// tip
		T::Balance,
		// who paid the fee - this is an option to allow for a Default impl.
		T::AccountId,
		// imbalance resulting from withdrawing the fee
		T::Balance,
	);

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		let (_tip, asset_id) = self.0;
		if let Some(asset_id) = asset_id {
			return Ok(Some(asset_id))
		}
		Ok(None)
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> TransactionValidity {
		let (tip, asset) = self.0;
		let final_fee = Pallet::<T>::withdraw_fee(who, call, info, len, tip, asset)?;
		Ok(ValidTransaction {
			priority: Pallet::<T>::get_priority(info, len, tip, final_fee),
			..Default::default()
		})
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		let (tip, asset) = self.0;
		let imbalance = Pallet::<T>::withdraw_fee(who, call, info, len, tip, asset)?;
		Ok((asset, tip, who.clone(), imbalance))
	}

	fn post_dispatch(
		maybe_pre: Option<Self::Pre>,
		info: &DispatchInfoOf<Self::Call>,
		post_info: &PostDispatchInfoOf<Self::Call>,
		len: usize,
		_result: &DispatchResult,
	) -> Result<(), TransactionValidityError> {
		if let Some((asset, tip, who, imbalance)) = maybe_pre {
			let actual_fee = Pallet::<T>::compute_actual_fee(len, info, post_info, tip);
			Pallet::<T>::settle_balance_fee(&who, info, post_info, actual_fee, tip, imbalance)?;
			Pallet::<T>::deposit_event(Event::<T>::TransactionFeePaid {
				who,
				actual_fee,
				tip,
				asset,
			});
		}
		Ok(())
	}
}
