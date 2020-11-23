#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use sp_arithmetic::FixedU128;
use sp_core::H256;
use sp_std::vec::Vec;
use pallet_polkadex::data_structure_rpc::LinkedPriceLevelRpc;
use pallet_polkadex::data_structure_rpc::MarketDataRpc;
use pallet_polkadex::data_structure_rpc::OrderbookRpc;
use pallet_polkadex::data_structure_rpc::ErrorRpc;
use pallet_polkadex::data_structure_rpc::OrderbookUpdates;




sp_api::decl_runtime_apis!{
	pub trait DexStorageApi {
		fn get_ask_level(trading_pair: H256) -> Result<Vec<FixedU128>,ErrorRpc>;

		fn get_bid_level(trading_pair: H256) -> Result<Vec<FixedU128>, ErrorRpc>;

	    fn get_price_level(trading_pair: H256) -> Result<Vec<LinkedPriceLevelRpc>, ErrorRpc>;

	    fn get_orderbook(trading_pair: H256) -> Result<OrderbookRpc, ErrorRpc>;

	    fn get_all_orderbook() -> Result<Vec<OrderbookRpc>, ErrorRpc>;

        fn get_market_info(trading_pair: H256,blocknum: u32) -> Result<MarketDataRpc, ErrorRpc>;

        fn get_orderbook_updates(trading_pair: H256) -> Result<OrderbookUpdates,ErrorRpc>;

	}
}