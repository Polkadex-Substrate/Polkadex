
//! Autogenerated weights for `pallet_grandpa`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2021-09-10, STEPS: `10`, REPEAT: 4, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 128

// Executed Command:
// ./target/release/polkadex-node
// benchmark
// --chain
// dev
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet=pallet_grandpa
// --extrinsic=*
// --steps
// 10
// --repeat
// 4
// --output=benchout/pallet_grandpa


#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for pallet_grandpa.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_grandpa::WeightInfo for WeightInfo<T> {
	fn check_equivocation_proof(_x: u32, ) -> Weight {
		(80_623_000 as Weight)
	}
	// Storage: Grandpa Stalled (r:0 w:1)
	fn note_stalled() -> Weight {
		(2_065_000 as Weight)
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
}
