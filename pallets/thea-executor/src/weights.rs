//! Autogenerated weights for `thea_executor`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-10-31, STEPS: `100`, REPEAT: `200`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-41-122`, CPU: `AMD EPYC 7571`
//! WASM-EXECUTION: `Compiled`, CHAIN: `None`, DB CACHE: 1024

// Executed Command:
// ./polkadex-node
// benchmark
// pallet
// --pallet
// thea-executor
// --steps
// 100
// --repeat
// 200
// --extrinsic
// *
// --output
// weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `thea_executor`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::WeightInfo for WeightInfo<T> {
	/// Storage: `TheaExecutor::WithdrawalFees` (r:0 w:1)
	/// Proof: `TheaExecutor::WithdrawalFees` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `r` is `[1, 1000]`.
	fn set_withdrawal_fee(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 27_500_000 picoseconds.
		Weight::from_parts(28_833_924, 0)
			.saturating_add(Weight::from_parts(0, 0))
			// Standard Error: 9
			.saturating_add(Weight::from_parts(73, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `TheaExecutor::Metadata` (r:0 w:1)
	/// Proof: `TheaExecutor::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `r` is `[1, 1000]`.
	fn update_asset_metadata(_r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 25_870_000 picoseconds.
		Weight::from_parts(27_213_487, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `TheaExecutor::ApprovedDeposits` (r:1 w:1)
	/// Proof: `TheaExecutor::ApprovedDeposits` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::Metadata` (r:1 w:0)
	/// Proof: `TheaExecutor::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(222), added: 2697, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(146), added: 2621, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:1 w:1)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 1000]`.
	fn claim_deposit(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `1555`
		//  Estimated: `5011`
		// Minimum execution time: 715_824_000 picoseconds.
		Weight::from_parts(730_233_094, 0)
			.saturating_add(Weight::from_parts(0, 5011))
			// Standard Error: 322
			.saturating_add(Weight::from_parts(4_304, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(4))
	}
	/// Storage: `TheaExecutor::RandomnessNonce` (r:1 w:1)
	/// Proof: `TheaExecutor::RandomnessNonce` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::PendingWithdrawals` (r:1 w:1)
	/// Proof: `TheaExecutor::PendingWithdrawals` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::Metadata` (r:1 w:0)
	/// Proof: `TheaExecutor::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::WithdrawalFees` (r:1 w:0)
	/// Proof: `TheaExecutor::WithdrawalFees` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(222), added: 2697, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(146), added: 2621, mode: `MaxEncodedLen`)
	/// Storage: `TheaExecutor::ReadyWithdrawals` (r:0 w:1)
	/// Proof: `TheaExecutor::ReadyWithdrawals` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `r` is `[1, 1000]`.
	fn withdraw(_r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `826`
		//  Estimated: `6196`
		// Minimum execution time: 291_352_000 picoseconds.
		Weight::from_parts(299_646_631, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(7))
	}
	/// Storage: `TheaExecutor::RandomnessNonce` (r:1 w:1)
	/// Proof: `TheaExecutor::RandomnessNonce` (`max_values`: Some(1), `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::PendingWithdrawals` (r:1 w:1)
	/// Proof: `TheaExecutor::PendingWithdrawals` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::Metadata` (r:1 w:0)
	/// Proof: `TheaExecutor::Metadata` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `TheaExecutor::WithdrawalFees` (r:1 w:0)
	/// Proof: `TheaExecutor::WithdrawalFees` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Asset` (r:1 w:1)
	/// Proof: `Assets::Asset` (`max_values`: None, `max_size`: Some(222), added: 2697, mode: `MaxEncodedLen`)
	/// Storage: `Assets::Account` (r:1 w:1)
	/// Proof: `Assets::Account` (`max_values`: None, `max_size`: Some(146), added: 2621, mode: `MaxEncodedLen`)
	/// Storage: `TheaExecutor::ReadyWithdrawals` (r:0 w:1)
	/// Proof: `TheaExecutor::ReadyWithdrawals` (`max_values`: None, `max_size`: None, mode: `Measured`)
	/// The range of component `r` is `[1, 1000]`.
	fn parachain_withdraw(_r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `793`
		//  Estimated: `6196`
		// Minimum execution time: 292_152_000 picoseconds.
		Weight::from_parts(298_959_519, 0)
			.saturating_add(Weight::from_parts(0, 6196))
			.saturating_add(T::DbWeight::get().reads(8))
			.saturating_add(T::DbWeight::get().writes(7))
	}
}
