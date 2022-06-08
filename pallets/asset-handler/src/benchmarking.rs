use super::*;
use crate::Pallet as Ethereum;
use codec::{Decode, Encode};
use frame_benchmarking::{account, benchmarks};
use frame_support::traits::fungibles::Mutate;
use frame_system::RawOrigin;
use sp_runtime::{traits::Bounded, AccountId32};

benchmark!{
    create_asset {
        let b in 0 .. 255;
        let chain_id = 1;
		let id = H160::from_slice(&[b as u8; 20]);
    }: _(RawOrigin::Root, chain_id, id)
}