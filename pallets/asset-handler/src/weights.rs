
//! Autogenerated weights for `asset_handler`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2022-10-20, STEPS: `10`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Ubuntu-2204-jammy-amd64-base`, CPU: `Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz`
//! EXECUTION: None, WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 1024

// Executed Command:
// ./polkadex-node
// benchmark
// pallet
// --pallet
// asset-handler
// --steps
// 10
// --repeat
// 20
// --extrinsic
// *
// --output
// assets_weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;
use crate::pallet::AssetHandlerWeightInfo;

/// Weight functions for `asset_handler`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> AssetHandlerWeightInfo for WeightInfo<T> {
	// Storage: ChainBridge AssetIdToResourceMap (r:1 w:1)
	// Storage: Assets Asset (r:1 w:1)
	/// The range of component `b` is `[0, 255]`.
	fn create_asset(b: u32, ) -> Weight {
		(18_648_000 as Weight)
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(T::DbWeight::get().reads(2 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: ChainBridge AssetIdToResourceMap (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	/// The range of component `b` is `[1, 1000]`.
	fn mint_asset(_b: u32, ) -> Weight {
		(30_486_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: AssetHandler BridgeDeactivated (r:0 w:1)
	fn set_bridge_status() -> Weight {
		(11_107_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: AssetHandler WithdrawalExecutionBlockDiff (r:0 w:1)
	fn set_block_delay() -> Weight {
		(11_144_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: AssetHandler BridgeFee (r:0 w:1)
	/// The range of component `m` is `[1, 100]`.
	/// The range of component `f` is `[1, 1000]`.
	fn update_fee(_m: u32, _f: u32, ) -> Weight {
		(11_868_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: AssetHandler AllowlistedToken (r:1 w:0)
	// Storage: ChainBridge ChainNonces (r:1 w:0)
	// Storage: AssetHandler BridgeDeactivated (r:1 w:0)
	// Storage: Assets Asset (r:1 w:1)
	// Storage: Assets Account (r:1 w:1)
	// Storage: AssetHandler PendingWithdrawals (r:1 w:1)
	// Storage: AssetHandler BridgeFee (r:1 w:0)
	// Storage: AssetHandler WithdrawalExecutionBlockDiff (r:1 w:0)
	/// The range of component `b` is `[10, 1000]`.
	/// The range of component `c` is `[1010, 2000]`.
	fn withdraw(_b: u32, _c: u32, ) -> Weight {
		(39_798_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(8 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
}
