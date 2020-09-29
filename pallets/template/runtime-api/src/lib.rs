#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use sp_arithmetic::FixedU128;
use sp_core::H256;
use sp_std::vec::Vec;
use pallet_template::LinkedPriceLevelRpc;
use pallet_template::MarketDataRpc;
use pallet_template::OrderbookRpc;
use pallet_template::Trait;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};


sp_api::decl_runtime_apis!{
	pub trait DexStorageApi {
		fn get_ask_level(trading_pair: H256) -> Vec<FixedU128>;

		fn get_bid_level(trading_pair: H256) -> Vec<FixedU128>;

	    fn get_price_level(trading_pair: H256) -> Vec<LinkedPriceLevelRpc>;

	    fn get_orderbook(trading_pair: H256) -> OrderbookRpc;

	    fn get_all_orderbook() -> Vec<OrderbookRpc>;

        fn get_market_info(trading_pair: H256,blocknum: u32) -> MarketDataRpc;
	}
}