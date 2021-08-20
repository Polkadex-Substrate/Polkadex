
//! Autogenerated weights for basic_channel::inbound
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 3.0.0
//! DATE: 2021-05-08, STEPS: `[50, ]`, REPEAT: 20, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("/tmp/snowbridge-benchmark-bNy/spec.json"), DB CACHE: 128

// Executed Command:
// target/release/snowbridge
// benchmark
// --chain
// /tmp/snowbridge-benchmark-bNy/spec.json
// --execution
// wasm
// --wasm-execution
// compiled
// --pallet
// basic_channel::inbound
// --extrinsic
// *
// --repeat
// 20
// --steps
// 50
// --output
// runtime/rococo/src/weights/basic_channel_inbound_weights.rs


#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for basic_channel::inbound.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> snowbridge_basic_channel::inbound::WeightInfo for WeightInfo<T> {
	fn submit() -> Weight {
		(176_439_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(6 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
}
