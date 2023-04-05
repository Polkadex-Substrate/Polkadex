
//! Autogenerated weights for `thea_staking`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-03-20, STEPS: `20`, REPEAT: 10, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! HOSTNAME: `Ubuntu-2204-jammy-amd64-base`, CPU: `Intel(R) Core(TM) i7-7700 CPU @ 3.60GHz`
//! EXECUTION: None, WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./polkadex-node
// benchmark
// pallet
// --chain
// dev
// --pallet
// thea_staking
// --extrinsic
// *
// --steps
// 20
// --repeat
// 10
// --output
// weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `thea_staking`.
pub struct StakeWeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> crate::pallet::TheaStakingWeightInfo for StakeWeightInfo<T> {
	// Storage: TheaStaking Stakinglimits (r:0 w:1)
	/// The range of component `a` is `[1, 10]`.
	/// The range of component `m` is `[100, 4294967295]`.
	fn set_staking_limits(a: u32, _m: u32, ) -> Weight {
		(3_454_000 as Weight)
			// Standard Error: 2_000
			.saturating_add((10_000 as Weight).saturating_mul(a as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: TheaStaking Candidates (r:1 w:1)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: TheaStaking CandidateToNetworkMapping (r:0 w:1)
	/// The range of component `a` is `[0, 255]`.
	/// The range of component `b` is `[0, 255]`.
	/// The range of component `m` is `[100, 4294967295]`.
	fn add_candidate(_a: u32, b: u32, _m: u32, ) -> Weight {
		(29_974_000 as Weight)
			// Standard Error: 0
			.saturating_add((2_000 as Weight).saturating_mul(b as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: TheaStaking Stakers (r:1 w:1)
	// Storage: TheaStaking CandidateToNetworkMapping (r:1 w:0)
	// Storage: TheaStaking Candidates (r:1 w:1)
	/// The range of component `m` is `[100, 4294967295]`.
	/// The range of component `k` is `[1, 255]`.
	/// The range of component `x` is `[1, 255]`.
	fn nominate(_m: u32, k: u32, _x: u32, ) -> Weight {
		(26_902_000 as Weight)
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(k as Weight))
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: TheaStaking CandidateToNetworkMapping (r:1 w:0)
	// Storage: TheaStaking Stakinglimits (r:1 w:0)
	// Storage: TheaStaking Candidates (r:1 w:1)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	// Storage: TheaStaking Stakers (r:0 w:1)
	/// The range of component `m` is `[100, 4294967295]`.
	/// The range of component `k` is `[1, 255]`.
	/// The range of component `x` is `[1, 255]`.
	fn bond(_m: u32, k: u32, x: u32, ) -> Weight {
		(37_234_000 as Weight)
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(k as Weight))
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(x as Weight))
			.saturating_add(T::DbWeight::get().reads(5 as Weight))
			.saturating_add(T::DbWeight::get().writes(4 as Weight))
	}
	// Storage: TheaStaking Stakers (r:1 w:1)
	// Storage: TheaStaking CandidateToNetworkMapping (r:1 w:0)
	// Storage: TheaStaking Candidates (r:1 w:1)
	// Storage: TheaStaking CurrentIndex (r:1 w:0)
	/// The range of component `m` is `[100, 4294967295]`.
	/// The range of component `k` is `[1, 255]`.
	/// The range of component `x` is `[1, 255]`.
	fn unbond(_m: u32, k: u32, _x: u32, ) -> Weight {
		(27_665_000 as Weight)
			// Standard Error: 0
			.saturating_add((1_000 as Weight).saturating_mul(k as Weight))
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: TheaStaking Stakers (r:1 w:1)
	// Storage: TheaStaking CurrentIndex (r:1 w:0)
	// Storage: Balances Reserves (r:1 w:1)
	// Storage: System Account (r:1 w:1)
	/// The range of component `m` is `[100000000, 4294967295]`.
	/// The range of component `k` is `[1, 255]`.
	/// The range of component `x` is `[1, 255]`.
	fn withdraw_unbonded(_m: u32, _k: u32, _x: u32, ) -> Weight {
		(32_902_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(4 as Weight))
			.saturating_add(T::DbWeight::get().writes(3 as Weight))
	}
	// Storage: TheaStaking Candidates (r:1 w:1)
	// Storage: TheaStaking InactiveCandidates (r:0 w:1)
	/// The range of component `m` is `[100, 4294967295]`.
	/// The range of component `k` is `[1, 255]`.
	fn remove_candidate(_m: u32, _k: u32, ) -> Weight {
		(18_897_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: TheaStaking ActiveNetworks (r:1 w:1)
	// Storage: Thea ActiveNetworks (r:0 w:1)
	/// The range of component `n` is `[1, 255]`.
	fn add_network(_n: u32, ) -> Weight {
		(14_592_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: TheaStaking ActiveNetworks (r:1 w:1)
	// Storage: Thea ActiveNetworks (r:0 w:1)
	/// The range of component `n` is `[1, 255]`.
	fn remove_network(_n: u32, ) -> Weight {
		(15_268_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(1 as Weight))
			.saturating_add(T::DbWeight::get().writes(2 as Weight))
	}
	// Storage: TheaStaking ActiveRelayers (r:1 w:0)
	// Storage: TheaStaking ReportedOffenders (r:1 w:0)
	// Storage: TheaStaking CommitedSlashing (r:1 w:1)
	/// The range of component `n` is `[1, 255]`.
	fn report_offence(_n: u32, ) -> Weight {
		(24_783_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
			.saturating_add(T::DbWeight::get().writes(1 as Weight))
	}
	// Storage: TheaStaking EraRewardPayout (r:1 w:0)
	// Storage: TheaStaking EraRewardPoints (r:1 w:0)
	// Storage: TheaStaking TotalElectedRelayers (r:1 w:0)
	/// The range of component `k` is `[1, 255]`.
	/// The range of component `m` is `[100, 4294967295]`.
	/// The range of component `x` is `[1, 255]`.
	fn stakers_payout(_k: u32, _m: u32, _x: u32, ) -> Weight {
		(23_379_000 as Weight)
			.saturating_add(T::DbWeight::get().reads(3 as Weight))
	}
}
