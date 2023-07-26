//! Autogenerated weights for `thea`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-07-22, STEPS: `100`, REPEAT: `200`, LOW RANGE: `[]`, HIGH RANGE: `[]`
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
	/// Storage: TheaExecutor ApprovedDeposits (r:1 w:1)
	/// Proof Skipped: TheaExecutor ApprovedDeposits (max_values: None, max_size: None, mode: Measured)
	/// Storage: Thea IncomingNonce (r:0 w:1)
	/// Proof Skipped: Thea IncomingNonce (max_values: None, max_size: None, mode: Measured)
	/// Storage: Thea IncomingMessages (r:0 w:1)
	/// Proof Skipped: Thea IncomingMessages (max_values: None, max_size: None, mode: Measured)
	/// The range of component `b` is `[0, 256]`.
	fn incoming_message(_b: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6`
		//  Estimated: `3471`
		// Minimum execution time: 13_024_000 picoseconds.
		Weight::from_parts(14_512_856, 0)
			.saturating_add(Weight::from_parts(0, 3471))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(3))
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
		// Minimum execution time: 298_663_000 picoseconds.
		Weight::from_parts(307_818_738, 0)
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
		// Minimum execution time: 2_501_000 picoseconds.
		Weight::from_parts(2_836_182, 0)
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
		// Minimum execution time: 2_512_000 picoseconds.
		Weight::from_parts(2_869_356, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Thea ActiveNetworks (r:1 w:1)
	/// Proof Skipped: Thea ActiveNetworks (max_values: Some(1), max_size: None, mode: Measured)
	fn add_thea_network() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `145`
		//  Estimated: `1630`
		// Minimum execution time: 5_443_000 picoseconds.
		Weight::from_parts(5_636_000, 0)
			.saturating_add(Weight::from_parts(0, 1630))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Thea ActiveNetworks (r:1 w:1)
	/// Proof Skipped: Thea ActiveNetworks (max_values: Some(1), max_size: None, mode: Measured)
	fn remove_thea_network() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `166`
		//  Estimated: `1651`
		// Minimum execution time: 6_194_000 picoseconds.
		Weight::from_parts(6_413_000, 0)
			.saturating_add(Weight::from_parts(0, 1651))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
