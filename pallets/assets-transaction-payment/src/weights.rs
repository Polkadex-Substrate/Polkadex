
//! Autogenerated weights for `pallet_assets_transaction_payment`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-03-22, STEPS: `100`, REPEAT: 200, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Ubuntu-2204-jammy-amd64-base`, CPU: `Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz`
//! EXECUTION: None, WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 1024

// Executed Command:
// ./polkadex-node
// benchmark
// pallet
// --pallet
// pallet_assets_transaction_payment
// --steps
// 100
// --repeat
// 200
// --extrinsic
// *
// --output
// patp_weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_assets_transaction_payment`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> super::pallet::PotpWeightInfo for WeightInfo<T> {
	// Storage: AssetsTransactionPayment AllowedAssets (r:1 w:1)
	/// The range of component `b` is `[0, 255]`.
	fn allow_list_token_for_fees(_b: u32, ) -> Weight {
		(4_417_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: AssetsTransactionPayment AllowedAssets (r:1 w:1)
	/// The range of component `b` is `[0, 255]`.
	fn block_token_for_fees(_b: u32, ) -> Weight {
		(5_203_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
