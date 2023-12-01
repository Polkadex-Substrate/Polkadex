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

pub mod types;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use crate::types::MarketMakerConfig;
	use frame_support::{
		pallet_prelude::*,
		sp_runtime::{
			traits::{AccountIdConversion, BlockNumberProvider},
			SaturatedConversion,
		},
		traits::{
			fungibles::{Create, Inspect, Mutate},
			tokens::{Fortitude, Precision, Preservation},
			Currency, ExistenceRequirement, ReservableCurrency,
		},
		transactional, PalletId,
	};
	use frame_system::pallet_prelude::*;
	use orderbook_primitives::{constants::UNIT_BALANCE, types::TradingPair, LiquidityMining};
	use polkadex_primitives::AssetId;
	use rust_decimal::{prelude::*, Decimal};
	use sp_core::blake2_128;
	use sp_runtime::traits::UniqueSaturatedInto;
	use std::ops::{Div, DivAssign, MulAssign};

	type BalanceOf<T> = <<T as Config>::NativeCurrency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: IsType<<Self as frame_system::Config>::RuntimeEvent> + From<Event<Self>>;

		/// Some type that implements the LiquidityMining traits
		type OCEX: LiquidityMining<Self::AccountId, BalanceOf<Self>>;

		/// Pool Accounts are derived from this
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Balances Pallet
		type NativeCurrency: Currency<Self::AccountId> + ReservableCurrency<Self::AccountId>;

		/// Assets Pallet
		type OtherAssets: Mutate<
				<Self as frame_system::Config>::AccountId,
				Balance = BalanceOf<Self>,
				AssetId = AssetId,
			> + Inspect<<Self as frame_system::Config>::AccountId>
			+ Create<<Self as frame_system::Config>::AccountId>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Pools
	#[pallet::storage]
	#[pallet::getter(fn pools)]
	pub(super) type Pools<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		TradingPair,
		Identity,
		T::AccountId,
		MarketMakerConfig<T::AccountId, BlockNumberFor<T>>,
		OptionQuery,
	>;

	#[pallet::event]
	pub enum Event<T: Config> {}

	#[pallet::error]
	pub enum Error<T> {
		/// Market is not registered with OCEX pallet
		UnknownMarket,
		/// Decimal Conversion error
		ConversionError,
		/// Commission should be between 0-1
		InvalidCommission,
		/// Exit fee should be between 0-1
		InvalidExitFee,
		/// Pool already exists
		PoolExists,
		/// There is not enough quote for given base amount
		NotEnoughQuoteAmount,
		/// Pool is not registered
		UnknownPool,
		/// Public deposits not allowed in this pool
		PublicDepositsNotAllowed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Register a new pool
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn register_pool(
			origin: OriginFor<T>,
			name: [u8; 10],
			market: TradingPair,
			commission: u128,
			exit_fee: u128,
			public_funds_allowed: bool,
		) -> DispatchResult {
			let market_maker = ensure_signed(origin)?;

			ensure!(!<Pools<T>>::contains_key(market, &market_maker), Error::<T>::PoolExists);
			// Check market is active
			ensure!(T::OCEX::is_registered_market(&market), Error::<T>::UnknownMarket);
			// Check if commission and exit fee are between 0-1
			let mut commission =
				Decimal::from_u128(commission).ok_or(Error::<T>::ConversionError)?;
			let mut exit_fee = Decimal::from_u128(exit_fee).ok_or(Error::<T>::ConversionError)?;
			// Convert to Polkadex UNIT
			commission.div_assign(Decimal::from(UNIT_BALANCE));
			exit_fee.div_assign(Decimal::from(UNIT_BALANCE));
			ensure!(
				Decimal::zero() <= commission && commission <= Decimal::one(),
				Error::<T>::InvalidCommission
			);
			ensure!(
				Decimal::zero() <= exit_fee && exit_fee <= Decimal::one(),
				Error::<T>::InvalidExitFee
			);
			// Create the a pool address with origin and market combo if it doesn't exist
			let (pool, share_id) = Self::create_pool_account(&market_maker, market);
			T::OtherAssets::create(AssetId::Asset(share_id), pool.clone(), false, Zero::zero())?;
			// Register on OCEX pallet
			T::OCEX::register_pool(pool.clone());
			// Start cycle
			let config = MarketMakerConfig {
				pool_id: pool,
				commission,
				exit_fee,
				public_funds_allowed,
				name,
				cycle_start_blk: frame_system::Pallet::<T>::current_block_number(),
				share_id: AssetId::Asset(share_id),
			};
			<Pools<T>>::insert(market, market_maker, config);
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
			base_amount: u128,      // Amount of base asset to deposit
			max_quote_amount: u128, // Max quote amount willing to deposit
		) -> DispatchResult {
			let lp = ensure_signed(origin)?;
			let config = <Pools<T>>::get(market, &market_maker).ok_or(Error::<T>::UnknownPool)?;
			if !config.public_funds_allowed {
				ensure!(lp == market_maker, Error::<T>::PublicDepositsNotAllowed);
			}

			let mut base_amount =
				Decimal::from_u128(base_amount).ok_or(Error::<T>::ConversionError)?;
			let mut max_quote_amount =
				Decimal::from_u128(max_quote_amount).ok_or(Error::<T>::ConversionError)?;
			// Convert to Polkadex UNIT
			base_amount.div_assign(Decimal::from(UNIT_BALANCE));
			max_quote_amount.div_assign(Decimal::from(UNIT_BALANCE));

			let average_price = T::OCEX::average_price(market);

			// Calculate the required quote asset
			let required_quote_amount = average_price.saturating_mul(base_amount);
			ensure!(required_quote_amount <= max_quote_amount, Error::<T>::NotEnoughQuoteAmount);

			Self::transfer_asset(&lp, &config.pool_id, base_amount, market.base)?;
			Self::transfer_asset(&lp, &config.pool_id, required_quote_amount, market.quote)?;

			T::OCEX::add_liquidity(market, config.pool_id, base_amount, required_quote_amount);

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
			shares: BalanceOf<T>,
		) -> DispatchResult {
			let lp = ensure_signed(origin)?;

			let config = <Pools<T>>::get(market, market_maker).ok_or(Error::<T>::UnknownPool)?;

			let total = T::OtherAssets::total_issuance(config.share_id.into());
			let burned_amt = T::OtherAssets::burn_from(
				config.share_id.into(),
				&lp,
				shares,
				Precision::Exact,
				Fortitude::Force,
			)?;
			// TODO: When it should be queued.
			T::OCEX::remove_liquidity(burned_amt, total);
			Ok(())
		}

		#[pallet::call_index(3)]
		#[pallet::weight(10000)]
		#[transactional]
		pub fn force_close_pool(
			origin: OriginFor<T>,
			market: TradingPair,
			market_maker: T::AccountId,
			shares: u128,
		) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(<Pools<T>>::contains_key(market, &market_maker), Error::<T>::UnknownPool);
			T::OCEX::force_close_pool(market, market_maker);
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn create_pool_account(
			maker: &T::AccountId,
			market: TradingPair,
		) -> (T::AccountId, u128) {
			let mut preimage = Vec::new();
			maker.encode_to(&mut preimage);
			preimage.append(&mut market.encode());
			let hash = blake2_128(&preimage);
			let shares_id = u128::from_le_bytes(hash);
			let pool_id = T::PalletId::get();
			(pool_id.into_sub_account_truncating(hash), shares_id)
		}

		fn transfer_asset(
			payer: &T::AccountId,
			payee: &T::AccountId,
			mut amount: Decimal,
			asset: AssetId,
		) -> DispatchResult {
			amount.mul_assign(Decimal::from(UNIT_BALANCE));
			let amount: BalanceOf<T> =
				amount.to_u128().ok_or(Error::<T>::ConversionError)?.saturated_into();
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
						id.into(),
						payer,
						payee,
						amount.unique_saturated_into(),
						Preservation::Preserve,
					)?;
				},
			}
			Ok(())
		}
	}
}
