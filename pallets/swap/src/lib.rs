// Copyright 2021 Parallel Finance Developer.
// This file is part of Parallel Finance.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
// http://www.apache.org/licenses/LICENSE-2.0

// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Automatic Market Maker (AMM)
//!
//! Given any [X, Y] asset pair, "base" is the `X` asset while "quote" is the `Y` asset.

#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
	dispatch::DispatchResult,
	log,
	pallet_prelude::*,
	require_transactional,
	traits::{
		fungibles::{Inspect, Mutate, Transfer},
		Get, IsType,
	},
	transactional, Blake2_128Concat, PalletId,
};
use frame_system::{ensure_signed, pallet_prelude::OriginFor};
use num_traits::{cast::ToPrimitive, CheckedDiv, CheckedMul};
use polkadex_primitives::Balance;
use sp_runtime::{
	traits::{AccountIdConversion, CheckedAdd, CheckedSub, One, Saturating, Zero},
	ArithmeticError, DispatchError, FixedPointNumber, FixedU128, Permill, SaturatedConversion,
};
use sp_std::{cmp::min, result::Result, vec::Vec};
use support::{ConvertToBigUint, Pool};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
#[cfg(test)]
pub(crate) mod mock;
#[cfg(test)]
mod tests;

pub mod weights;

pub use pallet::*;
// pub use weights::WeightInfo;

pub trait WeightInfo {
	fn add_liquidity() -> Weight;
	fn remove_liquidity() -> Weight;
	fn create_pool() -> Weight;
	fn update_protocol_fee() -> Weight;
	fn update_protocol_fee_receiver() -> Weight;
}

pub type Ratio = Permill;
pub type CurrencyId = u128;
pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
pub type AssetIdOf<T, I = ()> =
	<<T as Config<I>>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::AssetId;
pub type BalanceOf<T, I = ()> =
	<<T as Config<I>>::Assets as Inspect<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	pub type Amounts<T, I> = sp_std::vec::Vec<BalanceOf<T, I>>;

	#[pallet::config]
	pub trait Config<I: 'static = ()>: frame_system::Config {
		type RuntimeEvent: From<Event<Self, I>>
			+ IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Currency type for deposit/withdraw assets to/from amm
		/// module
		type Assets: Transfer<Self::AccountId, AssetId = CurrencyId, Balance = Balance>
			+ Inspect<Self::AccountId, AssetId = CurrencyId, Balance = Balance>
			+ Mutate<Self::AccountId, AssetId = CurrencyId, Balance = Balance>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type LockAccountId: Get<Self::AccountId>;

		/// Weight information for extrinsics in this pallet.
		// type AMMWeightInfo: WeightInfo;

		/// Specify which origin is allowed to create new pools.
		type CreatePoolOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Specify which origin is allowed to update fee receiver.
		type ProtocolFeeUpdateOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;

		/// Defines the fees taken out of each trade and sent back to the AMM pool,
		/// typically 0.3%.
		#[pallet::constant]
		type LpFee: Get<Ratio>;

		/// Minimum amount of liquidty needed to init a new pool
		/// this amount is burned when the pool is created.
		///
		/// It's important that we include this value in order to
		/// prevent attacks where a bad actor will create and
		/// remove pools with malious intentions. By requiring
		/// a `MinimumLiquidity`, a pool cannot be removed since
		/// a small amount of tokens are locked forever when liquidity
		/// is first added.
		#[pallet::constant]
		type MinimumLiquidity: Get<BalanceOf<Self, I>>;

		/// How many routes we support at most
		#[pallet::constant]
		type MaxLengthRoute: Get<u32>;

		#[pallet::constant]
		type GetNativeCurrencyId: Get<AssetIdOf<Self, I>>;
	}

	#[pallet::error]
	pub enum Error<T, I = ()> {
		/// Pool does not exist
		PoolDoesNotExist,
		/// Insufficient liquidity
		InsufficientLiquidity,
		/// Not an ideal price ratio
		NotAnIdealPrice,
		/// Pool does not exist
		PoolAlreadyExists,
		/// Insufficient amount out
		InsufficientAmountOut,
		/// Insufficient amount in
		InsufficientAmountIn,
		/// Insufficient supply out.
		InsufficientSupplyOut,
		/// Identical assets
		IdenticalAssets,
		/// LP token has already been minted
		LpTokenAlreadyExists,
		/// Conversion failure to u128
		ConversionToU128Failed,
		/// Protocol fee receiver not set
		ProtocolFeeReceiverNotSet,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (crate) fn deposit_event)]
	pub enum Event<T: Config<I>, I: 'static = ()> {
		/// Add liquidity into pool
		/// [sender, base_currency_id, quote_currency_id, base_amount_added, quote_amount_added,
		/// lp_token_id, new_base_amount, new_quote_amount]
		LiquidityAdded(
			T::AccountId,
			AssetIdOf<T, I>,
			AssetIdOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			AssetIdOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
		),
		/// Remove liquidity from pool
		/// [sender, base_currency_id, quote_currency_id, liquidity, base_amount_removed,
		/// quote_amount_removed, lp_token_id, new_base_amount, new_quote_amount]
		LiquidityRemoved(
			T::AccountId,
			AssetIdOf<T, I>,
			AssetIdOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			AssetIdOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
		),
		/// A Pool has been created
		/// [trader, currency_id_in, currency_id_out, lp_token_id]
		PoolCreated(T::AccountId, AssetIdOf<T, I>, AssetIdOf<T, I>, AssetIdOf<T, I>),
		/// Trade using liquidity
		/// [trader, currency_id_in, currency_id_out, amount_in, amount_out, lp_token_id,
		/// new_quote_amount, new_base_amount]
		Traded(
			T::AccountId,
			AssetIdOf<T, I>,
			AssetIdOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
			AssetIdOf<T, I>,
			BalanceOf<T, I>,
			BalanceOf<T, I>,
		),

		/// Protocol fee proportion of LP fee updated.
		ProtocolFeeUpdated(Ratio),

		/// Protocol fee receiver updated
		ProtocolFeeReceiverUpdated(T::AccountId),
	}

	#[pallet::pallet]
	pub struct Pallet<T, I = ()>(_);

	/// A bag of liquidity composed by two different assets
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub type Pools<T: Config<I>, I: 'static = ()> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		AssetIdOf<T, I>,
		Blake2_128Concat,
		AssetIdOf<T, I>,
		Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
		OptionQuery,
	>;

	/// How much the protocol is taking out of each trade.
	#[pallet::storage]
	#[pallet::getter(fn protocol_fee)]
	pub type ProtocolFee<T: Config<I>, I: 'static = ()> = StorageValue<_, Ratio, ValueQuery>;

	/// Who/where to send the protocol fees
	#[pallet::storage]
	pub type ProtocolFeeReceiver<T: Config<I>, I: 'static = ()> = StorageValue<_, T::AccountId>;

	#[pallet::call]
	impl<T: Config<I>, I: 'static> Pallet<T, I> {
		/// Allow users to add liquidity to a given pool.
		///
		/// # Parameters
		///
		/// * `pool`: Currency pool, in which liquidity will be added.
		/// * `liquidity_amounts`: Liquidity amounts to be added in pool.
		/// * `minimum_amounts`: specifying its "worst case" ratio when pool already exists.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::add_liquidity())]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			pair: (AssetIdOf<T, I>, AssetIdOf<T, I>),
			desired_amounts: (BalanceOf<T, I>, BalanceOf<T, I>),
			minimum_amounts: (BalanceOf<T, I>, BalanceOf<T, I>),
		) -> DispatchResultWithPostInfo {
			let who = ensure_signed(origin)?;
			let (is_inverted, base_asset, quote_asset) = Self::sort_assets(pair)?;

			let (base_amount, quote_amount) = if is_inverted {
				(desired_amounts.1, desired_amounts.0)
			} else {
				(desired_amounts.0, desired_amounts.1)
			};

			let (minimum_base_amount, minimum_quote_amount) = if is_inverted {
				(minimum_amounts.1, minimum_amounts.0)
			} else {
				(minimum_amounts.0, minimum_amounts.1)
			};

			Pools::<T, I>::try_mutate(
				base_asset,
				quote_asset,
				|pool| -> DispatchResultWithPostInfo {
					let pool = pool.as_mut().ok_or(Error::<T, I>::PoolDoesNotExist)?;

					let (ideal_base_amount, ideal_quote_amount) =
						Self::get_ideal_amounts(pool, (base_amount, quote_amount))?;

					ensure!(
						ideal_base_amount <= base_amount && ideal_quote_amount <= quote_amount,
						Error::<T, I>::InsufficientAmountIn
					);

					ensure!(
						ideal_base_amount >= minimum_base_amount &&
							ideal_quote_amount >= minimum_quote_amount,
						Error::<T, I>::NotAnIdealPrice
					);

					Self::do_mint_protocol_fee(pool)?;

					Self::do_add_liquidity(
						&who,
						pool,
						(ideal_base_amount, ideal_quote_amount),
						(base_asset, quote_asset),
					)?;

					log::trace!(
						target: "amm::add_liquidity",
						"who: {:?}, base_asset: {:?}, quote_asset: {:?}, ideal_amounts: {:?},\
						desired_amounts: {:?}, minimum_amounts: {:?}",
						&who,
						&base_asset,
						&quote_asset,
						&(ideal_base_amount, ideal_quote_amount),
						&desired_amounts,
						&minimum_amounts
					);

					Self::deposit_event(Event::<T, I>::LiquidityAdded(
						who,
						base_asset,
						quote_asset,
						ideal_base_amount,
						ideal_quote_amount,
						pool.lp_token_id,
						pool.base_amount,
						pool.quote_amount,
					));

					Ok(().into())
				},
			)
		}

		/// Allow users to remove liquidity from a given pool.
		///
		/// # Parameters
		///
		/// * `pair`: Currency pool, in which liquidity will be removed.
		/// * `liquidity`: liquidity to be removed from user's liquidity.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::remove_liquidity())]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			pair: (AssetIdOf<T, I>, AssetIdOf<T, I>),
			#[pallet::compact] liquidity: BalanceOf<T, I>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let (_, base_asset, quote_asset) = Self::sort_assets(pair)?;

			Pools::<T, I>::try_mutate(base_asset, quote_asset, |pool| -> DispatchResult {
				let pool = pool.as_mut().ok_or(Error::<T, I>::PoolDoesNotExist)?;

				Self::do_mint_protocol_fee(pool)?;

				let (base_amount_removed, quote_amount_removed) =
					Self::do_remove_liquidity(&who, pool, liquidity, (base_asset, quote_asset))?;

				log::trace!(
					target: "amm::remove_liquidity",
					"who: {:?}, base_asset: {:?}, quote_asset: {:?}, liquidity: {:?}",
					&who,
					&base_asset,
					&quote_asset,
					&liquidity
				);

				Self::deposit_event(Event::<T, I>::LiquidityRemoved(
					who,
					base_asset,
					quote_asset,
					liquidity,
					base_amount_removed,
					quote_amount_removed,
					pool.lp_token_id,
					pool.base_amount,
					pool.quote_amount,
				));

				Ok(())
			})
		}

		/// Create of a new pool, governance only.
		///
		/// # Parameters
		///
		/// * `pool`: Currency pool, in which liquidity will be added.
		/// * `liquidity_amounts`: Liquidity amounts to be added in pool.
		/// * `lptoken_receiver`: Allocate any liquidity tokens to lptoken_receiver.
		/// * `lp_token_id`: Liquidity pool share representative token.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::create_pool())]
		#[transactional]
		pub fn create_pool(
			origin: OriginFor<T>,
			pair: (AssetIdOf<T, I>, AssetIdOf<T, I>),
			liquidity_amounts: (BalanceOf<T, I>, BalanceOf<T, I>),
			lptoken_receiver: T::AccountId,
			lp_token_id: AssetIdOf<T, I>,
		) -> DispatchResultWithPostInfo {
			T::CreatePoolOrigin::ensure_origin(origin)?;

			let (is_inverted, base_asset, quote_asset) = Self::sort_assets(pair)?;
			ensure!(
				!Pools::<T, I>::contains_key(base_asset, quote_asset),
				Error::<T, I>::PoolAlreadyExists
			);

			let (base_amount, quote_amount) = if is_inverted {
				(liquidity_amounts.1, liquidity_amounts.0)
			} else {
				(liquidity_amounts.0, liquidity_amounts.1)
			};

			// check that this is a new asset to avoid using an asset that
			// already has tokens minted
			ensure!(
				T::Assets::total_issuance(lp_token_id).is_zero(),
				Error::<T, I>::LpTokenAlreadyExists
			);

			let mut pool = Pool::new(lp_token_id);

			Self::deposit_event(Event::<T, I>::PoolCreated(
				lptoken_receiver.clone(),
				base_asset,
				quote_asset,
				lp_token_id,
			));

			Self::do_add_liquidity(
				&lptoken_receiver,
				&mut pool,
				(base_amount, quote_amount),
				(base_asset, quote_asset),
			)?;

			Pools::<T, I>::insert(base_asset, quote_asset, pool);

			log::trace!(
				target: "amm::create_pool",
				"lptoken_receiver: {:?}, base_asset: {:?}, quote_asset: {:?}, base_amount: {:?}, quote_amount: {:?},\
				 liquidity_amounts: {:?}",
				&lptoken_receiver,
				&base_asset,
				&quote_asset,
				&base_amount,
				&quote_amount,
				&liquidity_amounts
			);

			Self::deposit_event(Event::<T, I>::LiquidityAdded(
				lptoken_receiver,
				base_asset,
				quote_asset,
				base_amount,
				quote_amount,
				pool.lp_token_id,
				pool.base_amount,
				pool.quote_amount,
			));

			Ok(().into())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::update_protocol_fee())]
		#[transactional]
		pub fn update_protocol_fee(
			origin: OriginFor<T>,
			protocol_fee: Ratio,
		) -> DispatchResultWithPostInfo {
			T::ProtocolFeeUpdateOrigin::ensure_origin(origin)?;
			ProtocolFee::<T, I>::put(protocol_fee);
			Self::deposit_event(Event::<T, I>::ProtocolFeeUpdated(protocol_fee));
			Ok(().into())
		}

		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::update_protocol_fee_receiver())]
		#[transactional]
		pub fn update_protocol_fee_receiver(
			origin: OriginFor<T>,
			protocol_fee_receiver: T::AccountId,
		) -> DispatchResultWithPostInfo {
			T::ProtocolFeeUpdateOrigin::ensure_origin(origin)?;
			ProtocolFeeReceiver::<T, I>::put(protocol_fee_receiver.clone());
			Self::deposit_event(Event::<T, I>::ProtocolFeeReceiverUpdated(protocol_fee_receiver));
			Ok(().into())
		}
	}
}

impl<T: Config<I>, I: 'static> Pallet<T, I> {
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account_truncating()
	}

	pub fn lock_account_id() -> T::AccountId {
		T::LockAccountId::get()
	}

	fn protolcol_fee_receiver() -> Result<T::AccountId, DispatchError> {
		Ok(ProtocolFeeReceiver::<T, I>::get().ok_or(Error::<T, I>::ProtocolFeeReceiverNotSet)?)
	}

	fn quote(
		base_amount: BalanceOf<T, I>,
		base_pool: BalanceOf<T, I>,
		quote_pool: BalanceOf<T, I>,
	) -> Result<BalanceOf<T, I>, DispatchError> {
		log::trace!(
			target: "amm::quote",
			"base_amount: {:?}, base_pool: {:?}, quote_pool: {:?}",
			&base_amount,
			&base_pool,
			&quote_pool
		);

		Ok(base_amount
			.get_big_uint()
			.checked_mul(&quote_pool.get_big_uint())
			.and_then(|r| r.checked_div(&base_pool.get_big_uint()))
			.and_then(|r| r.to_u128())
			.ok_or(ArithmeticError::Overflow)?)
	}

	#[allow(clippy::all)]
	fn sort_assets(
		(curr_a, curr_b): (AssetIdOf<T, I>, AssetIdOf<T, I>),
	) -> Result<(bool, AssetIdOf<T, I>, AssetIdOf<T, I>), DispatchError> {
		if curr_a > curr_b {
			return Ok((false, curr_a, curr_b))
		}

		if curr_a < curr_b {
			return Ok((true, curr_b, curr_a))
		}

		log::trace!(
			target: "amm::sort_assets",
			"pair: {:?}",
			&(curr_a, curr_b)
		);

		Err(Error::<T, I>::IdenticalAssets.into())
	}

	// given a pool, calculate the ideal liquidity amounts as a function of the current
	// pool reserves ratio
	#[allow(clippy::all)]
	fn get_ideal_amounts(
		pool: &Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
		(base_amount, quote_amount): (BalanceOf<T, I>, BalanceOf<T, I>),
	) -> Result<(BalanceOf<T, I>, BalanceOf<T, I>), DispatchError> {
		log::trace!(
			target: "amm::get_ideal_amounts",
			"pair: {:?}",
			&(base_amount, quote_amount)
		);

		if pool.is_empty() {
			return Ok((base_amount, quote_amount))
		}

		let ideal_quote_amount = Self::quote(base_amount, pool.base_amount, pool.quote_amount)?;
		if ideal_quote_amount <= quote_amount {
			Ok((base_amount, ideal_quote_amount))
		} else {
			let ideal_base_amount = Self::quote(quote_amount, pool.quote_amount, pool.base_amount)?;
			Ok((ideal_base_amount, quote_amount))
		}
	}

	fn protocol_fee_on() -> bool {
		!Self::protocol_fee().is_zero() && Self::protolcol_fee_receiver().is_ok()
	}

	fn get_protocol_fee_reciprocal_proportion() -> Result<BalanceOf<T, I>, DispatchError> {
		Ok(Self::protocol_fee().saturating_reciprocal_mul_floor::<BalanceOf<T, I>>(One::one()))
	}

	// given an input amount and a vector of assets, return a vector of output
	// amounts
	fn get_amounts_out(
		amount_in: BalanceOf<T, I>,
		path: Vec<AssetIdOf<T, I>>,
	) -> Result<Amounts<T, I>, DispatchError> {
		let mut amounts_out: Amounts<T, I> = Vec::new();
		amounts_out.resize(path.len(), 0u128);

		amounts_out[0] = amount_in;
		for i in 0..(path.len() - 1) {
			let (reserve_in, reserve_out) = Self::get_reserves(path[i], path[i + 1])?;
			let amount_out = Self::get_amount_out(amounts_out[i], reserve_in, reserve_out)?;
			amounts_out[i + 1] = amount_out;
		}

		Ok(amounts_out)
	}

	// given an output amount and a vector of assets, return a vector of required input
	// amounts to return the expected output amount
	fn get_amounts_in(
		amount_out: BalanceOf<T, I>,
		path: Vec<AssetIdOf<T, I>>,
	) -> Result<Amounts<T, I>, DispatchError> {
		let mut amounts_in: Amounts<T, I> = Vec::new();
		amounts_in.resize(path.len(), 0u128);
		let amount_len = amounts_in.len();

		amounts_in[amount_len - 1] = amount_out;
		for i in (1..(path.len())).rev() {
			let (reserve_in, reserve_out) = Self::get_reserves(path[i - 1], path[i])?;
			let amount_in = Self::get_amount_in(amounts_in[i], reserve_in, reserve_out)?;
			amounts_in[i - 1] = amount_in;
		}

		Ok(amounts_in)
	}

	// extract the reserves from a pool after sorting assets
	#[allow(clippy::all)]
	fn get_reserves(
		asset_in: AssetIdOf<T, I>,
		asset_out: AssetIdOf<T, I>,
	) -> Result<(BalanceOf<T, I>, BalanceOf<T, I>), DispatchError> {
		let (is_inverted, base_asset, quote_asset) = Self::sort_assets((asset_in, asset_out))?;

		let pool = Pools::<T, I>::try_get(base_asset, quote_asset)
			.map_err(|_err| Error::<T, I>::PoolDoesNotExist)?;

		if is_inverted {
			Ok((pool.quote_amount, pool.base_amount))
		} else {
			Ok((pool.base_amount, pool.quote_amount))
		}
	}

	// given an input amount of an asset and pair reserves, returns the maximum output amount of the
	// other asset
	//
	// amountIn = amountIn * (1 - fee_percent)
	// reserveIn * reserveOut = (reserveIn + amountIn) * (reserveOut - amountOut)
	// reserveIn * reserveOut = reserveIn * reserveOut + amountIn * reserveOut - (reserveIn +
	// amountIn) * amountOut amountIn * reserveOut = (reserveIn + amountIn) * amountOut
	//
	// amountOut = amountIn * reserveOut / (reserveIn + amountIn)
	fn get_amount_out(
		amount_in: BalanceOf<T, I>,
		reserve_in: BalanceOf<T, I>,
		reserve_out: BalanceOf<T, I>,
	) -> Result<BalanceOf<T, I>, DispatchError> {
		let fees = T::LpFee::get().mul_ceil(amount_in);

		let amount_in = amount_in.checked_sub(fees).ok_or(ArithmeticError::Underflow)?;

		let (amount_in, reserve_in, reserve_out) =
			(amount_in.get_big_uint(), reserve_in.get_big_uint(), reserve_out.get_big_uint());

		let numerator = amount_in.checked_mul(&reserve_out).ok_or(ArithmeticError::Overflow)?;

		let denominator = reserve_in.checked_add(&amount_in).ok_or(ArithmeticError::Overflow)?;

		let amount_out = numerator.checked_div(&denominator).ok_or(ArithmeticError::Underflow)?;

		log::trace!(
			target: "amm::get_amount_out",
			"amount_in: {:?}, reserve_in: {:?}, reserve_out: {:?}, fees: {:?}, numerator: {:?}, denominator: {:?},\
			 amount_out: {:?}",
			&amount_in,
			&reserve_in,
			&reserve_out,
			&fees,
			&numerator,
			&denominator,
			&amount_out
		);

		Ok(amount_out.to_u128().ok_or(ArithmeticError::Overflow)?)
	}

	// given an output amount of an asset and pair reserves, returns a required input amount of the
	// other asset
	//
	// amountOut = amountIn * reserveOut / (reserveIn + amountIn)
	// amountOut * reserveIn + amountOut * amountIn  = amountIn * reserveOut
	// amountOut * reserveIn = amountIn * (reserveOut - amountOut)
	//
	// amountIn = amountOut * reserveIn / (reserveOut - amountOut)
	//
	// Note: To make sure it greater than expected amount_out.
	// amountIn = (amountIn / (1 - fee_percent)) + **1**
	fn get_amount_in(
		amount_out: BalanceOf<T, I>,
		reserve_in: BalanceOf<T, I>,
		reserve_out: BalanceOf<T, I>,
	) -> Result<BalanceOf<T, I>, DispatchError> {
		ensure!(amount_out < reserve_out, Error::<T, I>::InsufficientSupplyOut);

		let (amount_out, reserve_in, reserve_out) =
			(amount_out.get_big_uint(), reserve_in.get_big_uint(), reserve_out.get_big_uint());

		let numerator = reserve_in.checked_mul(&amount_out).ok_or(ArithmeticError::Overflow)?;

		let denominator = reserve_out.checked_sub(&amount_out).ok_or(ArithmeticError::Underflow)?;

		let amount_in = numerator
			.checked_div(&denominator)
			.ok_or(ArithmeticError::Underflow)?
			.to_u128()
			.ok_or(ArithmeticError::Overflow)?;

		let fee_percent = Ratio::from_percent(100)
			.checked_sub(&T::LpFee::get())
			.ok_or(ArithmeticError::Underflow)?;

		log::trace!(
			target: "amm::get_amount_in",
			"amount_out: {:?}, reserve_in: {:?}, reserve_out: {:?}, numerator: {:?}, denominator: {:?}, amount_in: {:?}",
			&amount_out,
			&reserve_in,
			&reserve_out,
			&numerator,
			&denominator,
			&amount_in
		);

		Ok(fee_percent
			.saturating_reciprocal_mul_ceil(amount_in)
			.checked_add(One::one())
			.ok_or(ArithmeticError::Overflow)?)
	}

	// update internal twap price oracle by calculating the number of blocks elapsed
	// and update the pools cumulative prices
	fn do_update_oracle(
		pool: &mut Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
	) -> Result<(), DispatchError> {
		let block_timestamp = frame_system::Pallet::<T>::block_number();

		if pool.block_timestamp_last != block_timestamp {
			let time_elapsed: BalanceOf<T, I> =
				block_timestamp.saturating_sub(pool.block_timestamp_last).saturated_into();

			// compute by multiplying the numerator with the time elapsed
			let price0_fraction = FixedU128::saturating_from_rational(
				time_elapsed
					.get_big_uint()
					.checked_mul(&pool.quote_amount.get_big_uint())
					.ok_or(Error::<T, I>::ConversionToU128Failed)?
					.to_u128()
					.ok_or(ArithmeticError::Overflow)?,
				pool.base_amount,
			);
			let price1_fraction = FixedU128::saturating_from_rational(
				time_elapsed
					.get_big_uint()
					.checked_mul(&pool.base_amount.get_big_uint())
					.ok_or(Error::<T, I>::ConversionToU128Failed)?
					.to_u128()
					.ok_or(ArithmeticError::Overflow)?,
				pool.quote_amount,
			);

			// convert stored u128 into FixedU128 before add
			pool.price_0_cumulative_last = FixedU128::from_inner(pool.price_0_cumulative_last)
				.checked_add(&price0_fraction)
				.ok_or(ArithmeticError::Overflow)?
				.into_inner();

			pool.price_1_cumulative_last = FixedU128::from_inner(pool.price_1_cumulative_last)
				.checked_add(&price1_fraction)
				.ok_or(ArithmeticError::Overflow)?
				.into_inner();

			// updates timestamp last so `time_elapsed` is correctly calculated
			pool.block_timestamp_last = block_timestamp;
		}

		Ok(())
	}

	#[require_transactional]
	fn do_add_liquidity(
		who: &T::AccountId,
		pool: &mut Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
		(ideal_base_amount, ideal_quote_amount): (BalanceOf<T, I>, BalanceOf<T, I>),
		(base_asset, quote_asset): (AssetIdOf<T, I>, AssetIdOf<T, I>),
	) -> Result<(), DispatchError> {
		let total_supply = T::Assets::total_issuance(pool.lp_token_id);

		// lock a small amount of liquidity if the pool is first initialized
		let liquidity = if total_supply.is_zero() {
			T::Assets::mint_into(
				pool.lp_token_id,
				&Self::lock_account_id(),
				T::MinimumLiquidity::get(),
			)?;

			/*
			*----------------------------------------------------------------------------
						ideal_base_amount(x)    | ideal_quote_amount        | sqrt(z)
			 U128       2000                    |  1000                     | 1414
			 U128(Error)2000000000000000000000  |  1000000000000000000000   | None
			 BigUInt    2000000000000000000000  |  1000000000000000000000   | 1414213562373095047801
			-----------------------------------------------------------------------------
			*/

			ideal_base_amount
				.get_big_uint()
				.checked_mul(&ideal_quote_amount.get_big_uint())
				// loss of precision due to truncated sqrt
				.map(|r| r.sqrt())
				.and_then(|r| r.checked_sub(&T::MinimumLiquidity::get().get_big_uint()))
				.ok_or(Error::<T, I>::ConversionToU128Failed)?
				.to_u128()
				.ok_or(ArithmeticError::Underflow)?
		} else {
			min(
				ideal_base_amount
					.get_big_uint()
					.checked_mul(&total_supply.get_big_uint())
					.and_then(|r| r.checked_div(&pool.base_amount.get_big_uint()))
					.ok_or(Error::<T, I>::ConversionToU128Failed)?
					.to_u128()
					.ok_or(ArithmeticError::Underflow)?,
				ideal_quote_amount
					.get_big_uint()
					.checked_mul(&total_supply.get_big_uint())
					.and_then(|r| r.checked_div(&pool.quote_amount.get_big_uint()))
					.ok_or(Error::<T, I>::ConversionToU128Failed)?
					.to_u128()
					.ok_or(ArithmeticError::Underflow)?,
			)
		};

		// update reserves after liquidity calculation
		pool.base_amount = pool
			.base_amount
			.checked_add(ideal_base_amount)
			.ok_or(ArithmeticError::Overflow)?;
		pool.quote_amount = pool
			.quote_amount
			.checked_add(ideal_quote_amount)
			.ok_or(ArithmeticError::Overflow)?;

		T::Assets::mint_into(pool.lp_token_id, who, liquidity)?;

		T::Assets::transfer(
			base_asset,
			who,
			&Self::account_id(),
			ideal_base_amount,
			base_asset == T::GetNativeCurrencyId::get(), // should keep alive if is native
		)?;
		T::Assets::transfer(
			quote_asset,
			who,
			&Self::account_id(),
			ideal_quote_amount,
			quote_asset == T::GetNativeCurrencyId::get(), // should keep alive if is native
		)?;

		if Self::protocol_fee_on() {
			// we cannot hold k_last for really large values
			// we can hold two u128s instead
			pool.base_amount_last = pool.base_amount;
			pool.quote_amount_last = pool.quote_amount;
		}

		log::trace!(
			target: "amm::do_add_liquidity",
			"who: {:?}, total_supply: {:?}, liquidity: {:?}, base_asset: {:?}, quote_asset: {:?}, ideal_base_amount: {:?},\
			 ideal_quote_amount: {:?}",
			&who,
			&total_supply,
			&liquidity,
			&base_asset,
			&quote_asset,
			&ideal_base_amount,
			&ideal_quote_amount
		);

		Ok(())
	}

	#[allow(clippy::all)]
	fn calculate_reserves_to_remove(
		pool: &mut Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
		liquidity: BalanceOf<T, I>,
	) -> Result<(BalanceOf<T, I>, BalanceOf<T, I>), DispatchError> {
		let total_supply = T::Assets::total_issuance(pool.lp_token_id);
		let base_amount = liquidity
			.get_big_uint()
			.checked_mul(&pool.base_amount.get_big_uint())
			.and_then(|r| r.checked_div(&total_supply.get_big_uint()))
			.ok_or(Error::<T, I>::ConversionToU128Failed)?
			.to_u128()
			.ok_or(ArithmeticError::Underflow)?;
		let quote_amount = liquidity
			.get_big_uint()
			.checked_mul(&pool.quote_amount.get_big_uint())
			.and_then(|r| r.checked_div(&total_supply.get_big_uint()))
			.ok_or(Error::<T, I>::ConversionToU128Failed)?
			.to_u128()
			.ok_or(ArithmeticError::Underflow)?;

		Ok((base_amount, quote_amount))
	}
	#[allow(clippy::all)]
	#[require_transactional]
	fn do_remove_liquidity(
		who: &T::AccountId,
		pool: &mut Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
		liquidity: BalanceOf<T, I>,
		(base_asset, quote_asset): (AssetIdOf<T, I>, AssetIdOf<T, I>),
	) -> Result<(BalanceOf<T, I>, BalanceOf<T, I>), DispatchError> {
		let (base_amount, quote_amount) = Self::calculate_reserves_to_remove(pool, liquidity)?;

		pool.base_amount = pool
			.base_amount
			.checked_sub(base_amount)
			.ok_or(Error::<T, I>::InsufficientLiquidity)?;

		pool.quote_amount = pool
			.quote_amount
			.checked_sub(quote_amount)
			.ok_or(Error::<T, I>::InsufficientLiquidity)?;

		T::Assets::burn_from(pool.lp_token_id, who, liquidity)?;
		T::Assets::transfer(
			base_asset,
			&Self::account_id(),
			who,
			base_amount,
			base_asset == T::GetNativeCurrencyId::get(), // should keep alive if is native
		)?;
		T::Assets::transfer(
			quote_asset,
			&Self::account_id(),
			who,
			quote_amount,
			quote_asset == T::GetNativeCurrencyId::get(), // should keep alive if is native
		)?;

		if Self::protocol_fee_on() {
			// we cannot hold k_last for really large values
			// we can hold two u128s instead
			pool.base_amount_last = pool.base_amount;
			pool.quote_amount_last = pool.quote_amount;
		}

		log::trace!(
			target: "amm::do_remove_liquidity",
			"who: {:?}, liquidity: {:?}, base_asset: {:?}, quote_asset: {:?}, base_amount: {:?}, quote_amount: {:?}",
			&who,
			&liquidity,
			&base_asset,
			&quote_asset,
			&base_amount,
			&quote_amount
		);

		Ok((base_amount, quote_amount))
	}

	#[require_transactional]
	pub fn do_mint_protocol_fee(
		pool: &mut Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
	) -> Result<BalanceOf<T, I>, DispatchError> {
		let k_last = pool
			.base_amount_last
			.get_big_uint()
			.checked_mul(&pool.quote_amount_last.get_big_uint())
			.ok_or(ArithmeticError::Overflow)?;

		if !Self::protocol_fee_on() {
			// if fees are off and k_last is a value we need to reset it
			if !k_last.is_zero() {
				pool.base_amount_last = Zero::zero();
				pool.quote_amount_last = Zero::zero();
			}

			// if fees are off and k_last is zero return
			return Ok(Zero::zero())
		}

		// if the early exits do not return we know that k_last is not zero
		// and that protocol fees are on

		let root_k = pool
			.base_amount
			.get_big_uint()
			.checked_mul(&pool.quote_amount.get_big_uint())
			// loss of precision due to truncated sqrt
			.map(|r| r.sqrt())
			.ok_or(ArithmeticError::Overflow)?;

		let root_k_last = k_last
			// loss of precision due to truncated sqrt
			.sqrt();

		if root_k <= root_k_last {
			return Ok(Zero::zero())
		}

		let total_supply = T::Assets::total_issuance(pool.lp_token_id).get_big_uint();

		let numerator = root_k
			.checked_sub(&root_k_last)
			.and_then(|r| r.checked_mul(&total_supply))
			.ok_or(Error::<T, I>::ConversionToU128Failed)?;

		let scalar = Self::get_protocol_fee_reciprocal_proportion()?
			.checked_sub(One::one())
			.ok_or(ArithmeticError::Underflow)?
			.get_big_uint();

		let denominator = root_k
			.checked_mul(&scalar)
			.and_then(|r| r.checked_add(&root_k_last))
			.ok_or(Error::<T, I>::ConversionToU128Failed)?;

		let protocol_fees = numerator
			// loss of precision due to truncated division
			.checked_div(&denominator)
			.ok_or(ArithmeticError::Underflow)?
			.to_u128()
			.ok_or(ArithmeticError::Overflow)?;

		T::Assets::mint_into(pool.lp_token_id, &Self::protolcol_fee_receiver()?, protocol_fees)?;

		log::trace!(
			target: "amm::do_mint_protocol_fee",
			"root_k: {:?}, total_supply: {:?}, numerator: {:?}, denominator: {:?}, protocol_fees: {:?}",
			&root_k,
			&total_supply,
			&numerator,
			&denominator,
			&protocol_fees
		);
		Ok(protocol_fees)
	}

	fn do_swap(
		who: &T::AccountId,
		(asset_in, asset_out): (AssetIdOf<T, I>, AssetIdOf<T, I>),
		amount_in: BalanceOf<T, I>,
	) -> Result<BalanceOf<T, I>, DispatchError> {
		let (is_inverted, base_asset, quote_asset) = Self::sort_assets((asset_in, asset_out))?;

		Pools::<T, I>::try_mutate(
			base_asset,
			quote_asset,
			|pool| -> Result<BalanceOf<T, I>, DispatchError> {
				let pool = pool.as_mut().ok_or(Error::<T, I>::PoolDoesNotExist)?;

				let (supply_in, supply_out) = if is_inverted {
					(pool.quote_amount, pool.base_amount)
				} else {
					(pool.base_amount, pool.quote_amount)
				};

				ensure!(
					amount_in >= T::LpFee::get().saturating_reciprocal_mul_ceil(One::one()),
					Error::<T, I>::InsufficientAmountIn
				);
				ensure!(!supply_out.is_zero(), Error::<T, I>::InsufficientAmountOut);

				let amount_out = Self::get_amount_out(amount_in, supply_in, supply_out)?;

				let (new_supply_in, new_supply_out) = (
					supply_in.checked_add(amount_in).ok_or(ArithmeticError::Overflow)?,
					supply_out.checked_sub(amount_out).ok_or(ArithmeticError::Underflow)?,
				);

				if is_inverted {
					pool.quote_amount = new_supply_in;
					pool.base_amount = new_supply_out;
				} else {
					pool.base_amount = new_supply_in;
					pool.quote_amount = new_supply_out;
				}

				Self::do_update_oracle(pool)?;

				T::Assets::transfer(
					asset_in,
					who,
					&Self::account_id(),
					amount_in,
					asset_in == T::GetNativeCurrencyId::get(), // should keep alive if is native
				)?;
				T::Assets::transfer(
					asset_out,
					&Self::account_id(),
					who,
					amount_out,
					asset_out == T::GetNativeCurrencyId::get(), // should keep alive if is native
				)?;

				log::trace!(
					target: "amm::do_trade",
					"who: {:?}, asset_in: {:?}, asset_out: {:?}, amount_in: {:?}, amount_out: {:?}",
					&who,
					&asset_in,
					&asset_out,
					&amount_in,
					&amount_out,
				);

				Self::deposit_event(Event::<T, I>::Traded(
					who.clone(),
					asset_in,
					asset_out,
					amount_in,
					amount_out,
					pool.lp_token_id,
					pool.quote_amount,
					pool.base_amount,
				));

				Ok(amount_out)
			},
		)
	}
}

impl<T: Config<I>, I: 'static>
	support::AMM<AccountIdOf<T>, AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber> for Pallet<T, I>
{
	/// Based on the path specified and the available pool balances
	/// this will return the amounts outs when trading the specified
	/// amount in
	fn get_amounts_out(
		amount_in: BalanceOf<T, I>,
		path: Vec<AssetIdOf<T, I>>,
	) -> Result<Vec<BalanceOf<T, I>>, DispatchError> {
		let balances = Self::get_amounts_out(amount_in, path)?;
		Ok(balances)
	}

	/// Based on the path specified and the available pool balances
	/// this will return the amounts in needed to produce the specified
	/// amount out
	fn get_amounts_in(
		amount_out: BalanceOf<T, I>,
		path: Vec<AssetIdOf<T, I>>,
	) -> Result<Vec<BalanceOf<T, I>>, DispatchError> {
		let balances = Self::get_amounts_in(amount_out, path)?;
		Ok(balances)
	}

	/// Handles a "swap" on the AMM side for "who".
	/// This will move the `amount_in` funds to the AMM PalletId,
	/// trade `pair.0` to `pair.1` and return a result with the amount
	/// of currency that was sent back to the user.
	fn swap(
		who: &AccountIdOf<T>,
		pair: (AssetIdOf<T, I>, AssetIdOf<T, I>),
		amount_in: BalanceOf<T, I>,
	) -> Result<(), DispatchError> {
		Self::do_swap(who, pair, amount_in)?;
		Ok(())
	}

	/// Returns a vector of all of the pools in storage
	fn get_pools() -> Result<Vec<(AssetIdOf<T, I>, AssetIdOf<T, I>)>, DispatchError> {
		Ok(Pools::<T, I>::iter_keys().collect())
	}

	//just iterate now and require improve later when Pools increased
	/// Returns pool by lp_asset
	fn get_pool_by_lp_asset(
		asset_id: AssetIdOf<T, I>,
	) -> Option<(
		AssetIdOf<T, I>,
		AssetIdOf<T, I>,
		Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>,
	)> {
		for (base_asset, quote_asset, pool) in Pools::<T, I>::iter() {
			if pool.lp_token_id == asset_id {
				return Some((base_asset, quote_asset, pool))
			}
		}
		None
	}

	/// Returns pool by asset pair
	fn get_pool_by_asset_pair(
		(base_asset, quote_asset): (AssetIdOf<T, I>, AssetIdOf<T, I>),
	) -> Option<Pool<AssetIdOf<T, I>, BalanceOf<T, I>, T::BlockNumber>> {
		if let Ok((_, base_asset, quote_asset)) = Self::sort_assets((base_asset, quote_asset)) {
			return Pools::<T, I>::get(base_asset, quote_asset)
		}
		None
	}
}
