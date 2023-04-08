#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::unused_unit)]
#![allow(clippy::recursive_format_impl)]

pub mod payment;

use crate::{
	pallet::{AllowedAssets, Config, Event, Pallet},
	payment::OnChargeAssetTransaction,
};
use frame_support::{
	dispatch::{DispatchInfo, PostDispatchInfo},
	ensure,
	traits::{
		fungibles::{CreditOf, Inspect},
		IsType,
	},
};
use pallet_transaction_payment::OnChargeTransaction;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{DispatchInfoOf, Dispatchable, PostDispatchInfoOf, SignedExtension, Zero},
	transaction_validity::{
		InvalidTransaction, TransactionValidity, TransactionValidityError, ValidTransaction,
	},
	DispatchResult, FixedPointOperand,
};

#[cfg(test)]
mod mock;

#[cfg(test)]
mod test;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// Type aliases used for interaction with `OnChargeTransaction`.
pub(crate) type OnChargeTransactionOf<T> =
	<T as pallet_transaction_payment::Config>::OnChargeTransaction;
// Balance type alias.
pub(crate) type BalanceOf<T> = <OnChargeTransactionOf<T> as OnChargeTransaction<T>>::Balance;
// Liquidity info type alias.
pub(crate) type LiquidityInfoOf<T> =
	<OnChargeTransactionOf<T> as OnChargeTransaction<T>>::LiquidityInfo;

// Type alias used for interaction with fungibles (assets).
// Balance type alias.
pub type AssetBalanceOf<T> =
	<<T as Config>::Fungibles as Inspect<<T as frame_system::Config>::AccountId>>::Balance;
/// Asset id type alias.
pub(crate) type AssetIdOf<T> =
	<<T as Config>::Fungibles as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;

// Type aliases used for interaction with `OnChargeAssetTransaction`.
// Balance type alias.
pub(crate) type ChargeAssetBalanceOf<T> =
	<<T as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<T>>::Balance;
// Asset id type alias.
pub(crate) type ChargeAssetIdOf<T> =
	<<T as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<T>>::AssetId;
// Liquidity info type alias.
pub(crate) type ChargeAssetLiquidityOf<T> =
	<<T as Config>::OnChargeAssetTransaction as OnChargeAssetTransaction<T>>::LiquidityInfo;

/// Used to pass the initial payment info from pre- to post-dispatch.
#[derive(Encode, Decode, TypeInfo, Default)]
pub enum InitialPayment<T: Config> {
	/// No initial fee was payed.
	#[default]
	Nothing,
	/// The initial fee was payed in the native currency.
	Native(LiquidityInfoOf<T>),
	/// The initial fee was payed in an asset.
	Asset(CreditOf<T::AccountId, T::Fungibles>),
}

#[frame_support::pallet]
pub mod pallet {
	use crate::{payment::OnChargeAssetTransaction, AssetIdOf, BalanceOf};
	use frame_support::{pallet_prelude::*, traits::tokens::fungibles::Balanced};
	use frame_system::pallet_prelude::*;
	use sp_std::vec::Vec;

	pub trait PotpWeightInfo {
		fn allow_list_token_for_fees(_b: u32) -> Weight;
		fn block_token_for_fees(_b: u32) -> Weight;
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config:
		frame_system::Config + pallet_assets::Config + pallet_transaction_payment::Config
	{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The fungibles instance used to pay for transactions in assets.
		type Fungibles: Balanced<Self::AccountId>;
		/// The actual transaction charging logic that charges the fees.
		type OnChargeAssetTransaction: OnChargeAssetTransaction<Self>;
		/// Governance Origin
		type GovernanceOrigin: EnsureOrigin<<Self as frame_system::Config>::RuntimeOrigin>;
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	#[pallet::getter(fn allowed_assets)]
	pub type AllowedAssets<T: Config> = StorageValue<_, Vec<AssetIdOf<T>>, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		TransactionFeePaid {
			who: T::AccountId,
			actual_fee: BalanceOf<T>,
			tip: BalanceOf<T>,
		},
		AssetTxFeePaid {
			who: T::AccountId,
			actual_fee: BalanceOf<T>,
			tip: BalanceOf<T>,
			asset_id: AssetIdOf<T>,
		},
		InvalidAsset,
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Migration is not operational yet
		NotOperational,
		/// Token not allowlisted
		TokenNotAllowlisted,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// A way to add new tokens as payment for fees.
		///
		/// # Parameters
		///
		/// * `origin`: governance
		/// * `asset`: asset id in which fees will be accepted
		#[pallet::call_index(0)]
		#[pallet::weight(Weight::default())]
		pub fn allow_list_token_for_fees(
			origin: OriginFor<T>,
			asset: AssetIdOf<T>,
		) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<AllowedAssets<T>>::mutate(|allowed_assets| {
				if !allowed_assets.contains(&asset) {
					allowed_assets.push(asset);
				}
			});
			Ok(())
		}

		/// A way to remove tokens as payment for fees.
		///
		/// # Parameters
		///
		/// * `origin`: governance
		/// * `asset`: asset id in which fees should not be accepted anymore
		#[pallet::call_index(1)]
		#[pallet::weight(Weight::default())]
		pub fn block_token_for_fees(origin: OriginFor<T>, asset: AssetIdOf<T>) -> DispatchResult {
			T::GovernanceOrigin::ensure_origin(origin)?;
			<AllowedAssets<T>>::mutate(|allowed_assets| {
				if let Some(pos) = allowed_assets.iter().position(|&x| x == asset) {
					allowed_assets.remove(pos);
				}
			});
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {}
}

#[derive(Encode, Decode, Clone, Eq, PartialEq, TypeInfo)]
#[scale_info(skip_type_params(T))]
pub struct ChargeAssetTransactionPayment<T: Config> {
	pub asset_id: ChargeAssetIdOf<T>,
	pub tip: BalanceOf<T>,
	pub signature_scheme: u8,
}

impl<T: Config> ChargeAssetTransactionPayment<T>
where
	T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	AssetBalanceOf<T>: Send + Sync + FixedPointOperand,
	BalanceOf<T>: Send + Sync + FixedPointOperand + IsType<ChargeAssetBalanceOf<T>>,
	ChargeAssetIdOf<T>: Send + Sync + Zero,
	CreditOf<T::AccountId, T::Fungibles>: IsType<ChargeAssetLiquidityOf<T>>,
{
	/// Fee withdrawal logic that dispatches to either `OnChargeAssetTransaction` or
	/// `OnChargeTransaction`.
	fn withdraw_fee(
		&self,
		who: &T::AccountId,
		call: &T::RuntimeCall,
		info: &DispatchInfoOf<T::RuntimeCall>,
		len: usize,
	) -> Result<(BalanceOf<T>, InitialPayment<T>), TransactionValidityError> {
		let fee = pallet_transaction_payment::Pallet::<T>::compute_fee(len as u32, info, self.tip);
		ensure!(self.tip <= fee, TransactionValidityError::Invalid(InvalidTransaction::Payment));
		if fee.is_zero() {
			Ok((fee, InitialPayment::Nothing))
		} else if self.asset_id != Zero::zero() {
			T::OnChargeAssetTransaction::withdraw_fee(
				who,
				call,
				info,
				self.asset_id,
				fee.into(),
				self.tip.into(),
			)
			.map(|i| (fee, InitialPayment::Asset(i.into())))
		} else {
			// If the asset id is zero, then we treat that case as payment in PDEX,
			<OnChargeTransactionOf<T> as OnChargeTransaction<T>>::withdraw_fee(
				who, call, info, fee, self.tip,
			)
			.map(|i| (fee, InitialPayment::Native(i)))
			.map_err(|_| -> TransactionValidityError { InvalidTransaction::Payment.into() })
		}
	}
}

impl<T: Config> sp_std::fmt::Debug for ChargeAssetTransactionPayment<T> {
	#[cfg(feature = "std")]
	fn fmt(&self, f: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		write!(f, "ChargeTransactionPayment<{self:?}>")
	}
	#[cfg(not(feature = "std"))]
	fn fmt(&self, _: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		Ok(())
	}
}

impl<T: Config> SignedExtension for ChargeAssetTransactionPayment<T>
where
	T::RuntimeCall: Dispatchable<Info = DispatchInfo, PostInfo = PostDispatchInfo>,
	AssetBalanceOf<T>: Send + Sync + FixedPointOperand,
	BalanceOf<T>: Send + Sync + From<u64> + FixedPointOperand + IsType<ChargeAssetBalanceOf<T>>,
	ChargeAssetIdOf<T>: Send + Sync + Zero,
	CreditOf<T::AccountId, T::Fungibles>: IsType<ChargeAssetLiquidityOf<T>>,
{
	const IDENTIFIER: &'static str = "AssetsTransactionPayment";
	type AccountId = T::AccountId;
	type Call = T::RuntimeCall;
	type AdditionalSigned = u8;
	type Pre = (
		// tip
		BalanceOf<T>,
		// who paid the fee - this is an option to allow for a Default impl.
		T::AccountId,
		// imbalance resulting from withdrawing the fee
		InitialPayment<T>,
	);

	fn additional_signed(&self) -> Result<Self::AdditionalSigned, TransactionValidityError> {
		Ok(self.signature_scheme)
	}

	fn validate(
		&self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> TransactionValidity {
		use pallet_transaction_payment::ChargeTransactionPayment;
		let (fee, initial_payment) = self.withdraw_fee(who, call, info, len)?;
		// Check if the given asset is valid
		if let InitialPayment::Asset(asset) = initial_payment {
			let allowed_assets = <AllowedAssets<T>>::get();
			if !allowed_assets.contains(&asset.asset()) {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
			}
		} else {
			Pallet::<T>::deposit_event(Event::<T>::InvalidAsset);
			return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
		}
		let priority = ChargeTransactionPayment::<T>::get_priority(info, len, self.tip, fee);
		Ok(ValidTransaction { priority, ..Default::default() })
	}

	fn pre_dispatch(
		self,
		who: &Self::AccountId,
		call: &Self::Call,
		info: &DispatchInfoOf<Self::Call>,
		len: usize,
	) -> Result<Self::Pre, TransactionValidityError> {
		let (_fee, initial_payment) = self.withdraw_fee(who, call, info, len)?;
		// Check if the given asset is valid
		if let InitialPayment::Asset(asset) = &initial_payment {
			let allowed_assets = <AllowedAssets<T>>::get();
			if !allowed_assets.contains(&asset.asset()) {
				return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
			}
		}
		Ok((self.tip, who.clone(), initial_payment))
	}

	fn post_dispatch(
		pre: Option<Self::Pre>,
		info: &DispatchInfoOf<Self::Call>,
		post_info: &PostDispatchInfoOf<Self::Call>,
		len: usize,
		result: &DispatchResult,
	) -> Result<(), TransactionValidityError> {
		if let Some((tip, who, initial_payment)) = pre {
			match initial_payment {
				InitialPayment::Native(already_withdrawn) => {
					pallet_transaction_payment::ChargeTransactionPayment::<T>::post_dispatch(
						Some((tip, who, already_withdrawn)),
						info,
						post_info,
						len,
						result,
					)?;
				},
				InitialPayment::Asset(already_withdrawn) => {
					let actual_fee = pallet_transaction_payment::Pallet::<T>::compute_actual_fee(
						len as u32, info, post_info, tip,
					);
					let asset_id = already_withdrawn.asset();
					T::OnChargeAssetTransaction::correct_and_deposit_fee(
						&who,
						info,
						post_info,
						actual_fee.into(),
						tip.into(),
						already_withdrawn.into(),
					)?;
					Pallet::<T>::deposit_event(Event::<T>::AssetTxFeePaid {
						who,
						actual_fee,
						tip,
						asset_id,
					});
				},
				InitialPayment::Nothing => {
					// `actual_fee` should be zero here for any signed extrinsic. It would be
					// non-zero here in case of unsigned extrinsics as they don't pay fees but
					// `compute_actual_fee` is not aware of them. In both cases it's fine to just
					// move ahead without adjusting the fee, though, so we do nothing.
					return Err(TransactionValidityError::Invalid(InvalidTransaction::Payment))
				},
			}
		}
		Ok(())
	}
}
