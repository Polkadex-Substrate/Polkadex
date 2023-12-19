use frame_support::dispatch::DispatchResult;
use rust_decimal::Decimal;
use orderbook_primitives::traits::LiquidityMiningCrowdSourcePallet;
use orderbook_primitives::types::TradingPair;
use crate::pallet::{Config, Pallet};

impl<T: Config> LiquidityMiningCrowdSourcePallet<T::AccountId> for Pallet<T> {
    fn add_liquidity_success( market: TradingPair,pool: &T::AccountId, lp: &T::AccountId, shared_issued: Decimal, price: Decimal, total_inventory_in_quote: Decimal) -> DispatchResult {
        todo!()
    }

    fn remove_liquidity_success(  market: TradingPair,pool: &T::AccountId, lp: &T::AccountId, base_free: Decimal, quote_free: Decimal) -> DispatchResult {
        todo!()
    }

    fn remove_liquidity_failed(  market: TradingPair,pool: &T::AccountId, lp: &T::AccountId, burn_frac: Decimal, base_free: Decimal, quote_free: Decimal, base_required: Decimal, quote_required: Decimal) -> DispatchResult {
        todo!()
    }

    fn pool_force_close_success(market: TradingPair, pool: &T::AccountId, base_freed: Decimal, quote_freed: Decimal) -> DispatchResult {
        todo!()
    }

    fn stop_accepting_lmp_withdrawals(epoch: u16) {
        todo!()
    }
}