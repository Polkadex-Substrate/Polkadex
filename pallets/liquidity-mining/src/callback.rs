use crate::pallet::{Config, Error, Event, Pallet, Pools};
use frame_support::{dispatch::DispatchResult, traits::fungibles::Mutate};
use orderbook_primitives::{traits::LiquidityMiningCrowdSourcePallet, types::TradingPair};
use polkadex_primitives::UNIT_BALANCE;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use sp_runtime::SaturatedConversion;

impl<T: Config> LiquidityMiningCrowdSourcePallet<T::AccountId> for Pallet<T> {
	fn add_liquidity_success(
		market: TradingPair,
		pool: &T::AccountId,
		lp: &T::AccountId,
		shared_issued: Decimal,
		price: Decimal,
		total_inventory_in_quote: Decimal,
	) -> DispatchResult {
		let pool_config = <Pools<T>>::get(market, pool).ok_or(Error::<T>::UnknownPool)?;
		let new_shared_issued = shared_issued
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?;
		let price = price
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?;
		let total_inventory_in_quote = total_inventory_in_quote
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?;
		T::OtherAssets::mint_into(pool_config.share_id, lp, new_shared_issued.saturated_into())?;
		Self::deposit_event(Event::<T>::LiquidityAdded {
			market,
			pool: pool.clone(),
			lp: lp.clone(),
			shares: new_shared_issued.saturated_into(),
			share_id: pool_config.share_id,
			price,
			total_inventory_in_quote,
		});
		Ok(())
	}

	fn remove_liquidity_success(
		market: TradingPair,
		pool: &T::AccountId,
		lp: &T::AccountId,
		base_free: Decimal,
		quote_free: Decimal,
	) -> DispatchResult {
		Self::transfer_asset(pool, lp, base_free, market.base)?;
		Self::transfer_asset(pool, lp, quote_free, market.quote)?;
		Self::deposit_event(Event::<T>::LiquidityRemoved {
			market,
			pool: pool.clone(),
			lp: lp.clone(),
		});
		Ok(())
	}

	fn remove_liquidity_failed(
		market: TradingPair,
		pool: &T::AccountId,
		lp: &T::AccountId,
		burn_frac: Decimal,
		base_free: Decimal,
		quote_free: Decimal,
		base_required: Decimal,
		quote_required: Decimal,
	) -> DispatchResult {
		let burn_frac = burn_frac
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		let base_free = base_free
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		let quote_free = quote_free
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		let base_required = base_required
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		let quote_required = quote_required
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		Self::deposit_event(Event::<T>::LiquidityRemovalFailed {
			market,
			pool: pool.clone(),
			lp: lp.clone(),
			burn_frac,
			base_free,
			quote_free,
			base_required,
			quote_required,
		});
		Ok(())
	}

	fn pool_force_close_success(
		market: TradingPair,
		pool: &T::AccountId,
		base_freed: Decimal,
		quote_freed: Decimal,
	) -> DispatchResult {
		let mut pool_config = <Pools<T>>::get(market, pool).ok_or(Error::<T>::UnknownPool)?;
		pool_config.force_closed = true;
		<Pools<T>>::insert(market, pool, pool_config);
		let base_freed = base_freed
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		let quote_freed = quote_freed
			.saturating_mul(Decimal::from(UNIT_BALANCE))
			.to_u128()
			.ok_or(Error::<T>::ConversionError)?
			.saturated_into();
		Self::deposit_event(Event::<T>::PoolForceClosed {
			market,
			pool: pool.clone(),
			base_freed,
			quote_freed,
		});
		Ok(())
	}

	fn stop_accepting_lmp_withdrawals(epoch: u16) {
		todo!()
	}
}
