#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use sp_arithmetic::FixedU128;
use sp_core::H256;
use sp_std::vec::Vec;
use pallet_template::LinkedPriceLevel;
use pallet_template::Trait;


sp_api::decl_runtime_apis! {
	pub trait DexStorageApi<K> where K:Trait {
		fn get_ask_level(trading_pair: H256) -> Vec<FixedU128>;

		fn get_bid_level(trading_pair: H256) -> Vec<FixedU128>;

		fn get_price_level(trading_pair: H256) -> LinkedPriceLevel<K>;
	}
}