#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::unnecessary_mut_passed)]

use sp_arithmetic::{FixedPointNumber, FixedU128};
use sp_runtime::traits::Hash;

sp_api::decl_runtime_apis! {
	pub trait DexStorageApi {
		fn get_ask_level(trading_pair: Hash) -> Vec<FixedU128>;
	}
}