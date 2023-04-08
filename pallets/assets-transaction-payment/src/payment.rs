// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

///! Traits and default implementation for paying transaction fees in assets.
use super::*;
use crate::Config;

use frame_support::{
	traits::{
		fungibles::{Balanced, CreditOf, Inspect},
		tokens::{BalanceConversion, WithdrawConsequence},
		Currency, OnUnbalanced,
	},
	unsigned::TransactionValidityError,
};
use parity_scale_codec::FullCodec;
use scale_info::TypeInfo;
use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, DispatchInfoOf, MaybeSerializeDeserialize, One, PostDispatchInfoOf,
		Zero,
	},
	transaction_validity::InvalidTransaction,
	SaturatedConversion,
};
use sp_std::{fmt::Debug, marker::PhantomData};

/// Handle withdrawing, refunding and depositing of transaction fees.
pub trait OnChargeAssetTransaction<T: Config> {
	/// The underlying integer type in which fees are calculated.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default
		+ TypeInfo;
	/// The type used to identify the assets used for transaction payment.
	type AssetId: FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default + Eq + TypeInfo;
	/// The type used to store the intermediate values between pre- and post-dispatch.
	type LiquidityInfo;

	/// Before the transaction is executed the payment of the transaction fees needs to be secured.
	///
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		call: &T::RuntimeCall,
		dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		asset_id: Self::AssetId,
		fee: Self::Balance,
		tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError>;

	/// After the transaction was executed the actual fee can be calculated.
	/// This function should refund any overpaid fees and optionally deposit
	/// the corrected amount.
	///
	/// Note: The `fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		corrected_fee: Self::Balance,
		tip: Self::Balance,
		already_withdrawn: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError>;
}

pub type NegativeImbalanceOf<T> = <pallet_balances::Pallet<T> as Currency<
	<T as frame_system::Config>::AccountId,
>>::NegativeImbalance;

/// Allows specifying what to do with the withdrawn asset fees.
pub trait HandleSwap<T: Config + pallet_balances::Config> {
	/// Swap alternate token for PDEX
	fn swap(credit: CreditOf<T::AccountId, T::Fungibles>) -> NegativeImbalanceOf<T>;
}

/// Default implementation that just drops the credit according to the `OnDrop` in the underlying
/// imbalance type.
impl<T: Config + pallet_balances::Config> HandleSwap<T> for () {
	fn swap(credit: CreditOf<T::AccountId, T::Fungibles>) -> NegativeImbalanceOf<T> {
		// FIXME: Is there a better way to convert here?
		NegativeImbalanceOf::new(credit.peek().saturated_into::<u128>().saturated_into())
	}
}

/// Implements the asset transaction for a balance to asset converter (implementing
/// [`BalanceConversion`]) and a credit handler (implementing [`HandleCredit`]).
///
/// The credit handler is given the complete fee in terms of the asset used for the transaction.
pub struct FungiblesAdapter<CON, HC, OU>(PhantomData<(CON, HC, OU)>);

/// Default implementation for a runtime instantiating this pallet, a balance to asset converter and
/// a credit handler.
impl<T, CON, HC, OU> OnChargeAssetTransaction<T> for FungiblesAdapter<CON, HC, OU>
where
	T: Config + pallet_balances::Config,
	OU: OnUnbalanced<NegativeImbalanceOf<T>>,
	CON: BalanceConversion<BalanceOf<T>, AssetIdOf<T>, AssetBalanceOf<T>>,
	HC: HandleSwap<T>,
	AssetIdOf<T>: FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default + Eq + TypeInfo,
	OU: frame_support::traits::OnUnbalanced<pallet_balances::NegativeImbalance<T>>,
{
	type Balance = BalanceOf<T>;
	type AssetId = AssetIdOf<T>;
	type LiquidityInfo = CreditOf<T::AccountId, T::Fungibles>;

	/// Withdraw the predicted fee from the transaction origin.
	///
	/// Note: The `fee` already includes the `tip`.
	fn withdraw_fee(
		who: &T::AccountId,
		_call: &T::RuntimeCall,
		_info: &DispatchInfoOf<T::RuntimeCall>,
		asset_id: Self::AssetId,
		fee: Self::Balance,
		_tip: Self::Balance,
	) -> Result<Self::LiquidityInfo, TransactionValidityError> {
		// We don't know the precision of the underlying asset. Because the converted fee could be
		// less than one (e.g. 0.5) but gets rounded down by integer division we introduce a minimum
		// fee.
		let min_converted_fee = if fee.is_zero() { Zero::zero() } else { One::one() };
		let converted_fee = CON::to_asset_balance(fee, asset_id)
			.map_err(|_| TransactionValidityError::from(InvalidTransaction::Payment))?
			.max(min_converted_fee);
		let can_withdraw =
			<T::Fungibles as Inspect<T::AccountId>>::can_withdraw(asset_id, who, converted_fee);

		if !matches!(can_withdraw, WithdrawConsequence::Success) {
			return Err(InvalidTransaction::Payment.into())
		}
		<T::Fungibles as Balanced<T::AccountId>>::withdraw(asset_id, who, converted_fee)
			.map_err(|_err| TransactionValidityError::from(InvalidTransaction::Payment))
	}

	/// Hand the fee and the tip over to the `[HandleCredit]` implementation.
	/// Since the predicted fee might have been too high, parts of the fee may be refunded.
	///
	/// Note: The `corrected_fee` already includes the `tip`.
	fn correct_and_deposit_fee(
		who: &T::AccountId,
		_dispatch_info: &DispatchInfoOf<T::RuntimeCall>,
		_post_info: &PostDispatchInfoOf<T::RuntimeCall>,
		corrected_fee: Self::Balance,
		_tip: Self::Balance,
		paid: Self::LiquidityInfo,
	) -> Result<(), TransactionValidityError> {
		let min_converted_fee = if corrected_fee.is_zero() { Zero::zero() } else { One::one() };
		// Convert the corrected fee into the asset used for payment
		let converted_fee = CON::to_asset_balance(corrected_fee, paid.asset())
			.map_err(|_| -> TransactionValidityError { InvalidTransaction::Payment.into() })?
			.max(min_converted_fee);
		// Calculate how much refund we should return.
		let (final_fee, refund) = paid.split(converted_fee);
		// Refund to the account that paid the fees. If this fails, the account might have dropped
		// below the existential balance. In that case we don't refund anything.
		let _ = <T::Fungibles as Balanced<T::AccountId>>::resolve(who, refund);
		// Swap token for alternate currency
		let fee_in_pdex = HC::swap(final_fee);
		// Handle the final fee, e.g. by transferring to the block author or burning.
		OU::on_unbalanced(fee_in_pdex);
		Ok(())
	}
}
