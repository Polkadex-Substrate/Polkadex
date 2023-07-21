//! Autogenerated weights for `thea`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-07-20, STEPS: `100`, REPEAT: `200`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-5-61`, CPU: `Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz`
//! EXECUTION: None, WASM-EXECUTION: Compiled, CHAIN: None, DB CACHE: 1024

// Executed Command:
// ./polkadex-node
// benchmark
// pallet
// --pallet
// thea
// --steps
// 100
// --repeat
// 200
// --extrinsic
// *
// --output
// thea_weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `thea`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::TheaWeightInfo for WeightInfo<T> {
	/// Storage: Thea IncomingNonce (r:0 w:1)
	/// Proof Skipped: Thea IncomingNonce (max_values: None, max_size: None, mode: Measured)
	/// Storage: Thea IncomingMessages (r:0 w:1)
	/// Proof Skipped: Thea IncomingMessages (max_values: None, max_size: None, mode: Measured)
	/// The range of component `b` is `[0, 256]`.
	fn incoming_message(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 18_738_000 picoseconds.
		Weight::from_parts(22_767_276, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: Thea OutgoingNonce (r:1 w:1)
	/// Proof Skipped: Thea OutgoingNonce (max_values: None, max_size: None, mode: Measured)
	/// Storage: Thea ValidatorSetId (r:1 w:0)
	/// Proof Skipped: Thea ValidatorSetId (max_values: Some(1), max_size: None, mode: Measured)
	/// Storage: Thea OutgoingMessages (r:0 w:1)
	/// Proof Skipped: Thea OutgoingMessages (max_values: None, max_size: None, mode: Measured)
	/// The range of component `b` is `[0, 256]`.
	fn send_thea_message(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `145`
		//  Estimated: `3610`
		// Minimum execution time: 215_731_000 picoseconds.
		Weight::from_parts(226_460_727, 0)
			.saturating_add(Weight::from_parts(0, 3610))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: Thea IncomingNonce (r:0 w:1)
	/// Proof Skipped: Thea IncomingNonce (max_values: None, max_size: None, mode: Measured)
	/// The range of component `b` is `[1, 4294967295]`.
	fn update_incoming_nonce(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_563_000 picoseconds.
		Weight::from_parts(2_958_753, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Thea OutgoingNonce (r:0 w:1)
	/// Proof Skipped: Thea OutgoingNonce (max_values: None, max_size: None, mode: Measured)
	/// The range of component `b` is `[1, 4294967295]`.
	fn update_outgoing_nonce(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 2_547_000 picoseconds.
		Weight::from_parts(2_933_959, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
