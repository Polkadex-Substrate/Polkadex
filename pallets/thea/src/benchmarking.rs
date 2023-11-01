// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Benchmarking setup for pallet-ocex
#![cfg(feature = "runtime-benchmarks")]

use super::*;
use frame_benchmarking::benchmarks;
use frame_support::BoundedVec;
use frame_system::RawOrigin;
use parity_scale_codec::Decode;
use sp_std::collections::btree_set::BTreeSet;

fn generate_deposit_payload<T: Config>() -> Vec<Deposit<T::AccountId>> {
	sp_std::vec![Deposit {
		id: H256::zero().0.to_vec(),
		recipient: T::AccountId::decode(&mut &[0u8; 32][..]).unwrap(),
		asset_id: 0,
		amount: 0,
		extra: Vec::new(),
	}]
}

benchmarks! {
	incoming_message {
		let b in 0 .. 256; // keep withing u8 range
		let key = <T as crate::Config>::TheaId::generate_pair(None);
		let message = Message {
			block_no: u64::MAX,
			nonce: 1,
			data: generate_deposit_payload::<T>().encode(),
			network: 0u8,
			is_key_change: false,
			validator_set_id: 0,
		};
		let signature = key.sign(&message.encode()).unwrap();

		let mut set: BoundedVec<<T as crate::Config>::TheaId, <T as crate::Config>::MaxAuthorities> = BoundedVec::with_bounded_capacity(1);
		set.try_push(key).unwrap();
		<Authorities::<T>>::insert(0, set);
	}: _(RawOrigin::None, message, vec!((0, signature.into())))
	verify {
		assert!(<IncomingNonce::<T>>::get(0) == 1);
		assert!(<IncomingMessages::<T>>::iter().count() == 1);
	}

	send_thea_message {
		let b in 0 .. 256; // keep within u8 bounds
		let key =  <T as crate::Config>::TheaId::generate_pair(None);
		let network = b as u8;
		let data = [b as u8; 1_048_576].to_vec(); // 10MB
		let mut set: BoundedVec<<T as crate::Config>::TheaId, <T as crate::Config>::MaxAuthorities> = BoundedVec::with_bounded_capacity(1);
		set.try_push(key).unwrap();
		<Authorities::<T>>::insert(0, set);
	}: _(RawOrigin::Root, data, network)
	verify {
		assert!(<OutgoingNonce::<T>>::get(network) == 1);
		assert!(<OutgoingMessages::<T>>::iter().count() == 1);
	}

	update_incoming_nonce {
		let b in 1 .. u32::MAX;
		let network = 0;
		let nonce: u64 = b.into();
	}: _(RawOrigin::Root, nonce, network)
	verify {
		assert!(<IncomingNonce::<T>>::get(network) == nonce);
	}

	update_outgoing_nonce {
		let b in 1 .. u32::MAX;
		let network = 0;
		let nonce: u64 = b.into();
	}: _(RawOrigin::Root, nonce, network)
	verify {
		assert!(<OutgoingNonce::<T>>::get(network) == nonce);
	}

	add_thea_network {
		let network: u8 = 2;
	}: _(RawOrigin::Root, network)
	verify {
		let active_list = <ActiveNetworks<T>>::get();
		assert!(active_list.contains(&network));
	}

	remove_thea_network {
		let network: u8 = 2;
		let mut active_list = BTreeSet::new();
		active_list.insert(network);
		<ActiveNetworks<T>>::put(active_list);
	}: _(RawOrigin::Root, network)
	verify {
		let active_list = <ActiveNetworks<T>>::get();
		assert!(!active_list.contains(&network));
	}
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;
use sp_core::H256;
use thea_primitives::types::Deposit;

#[cfg(test)]
impl_benchmark_test_suite!(Thea, crate::mock::new_test_ext(), crate::mock::Test);
