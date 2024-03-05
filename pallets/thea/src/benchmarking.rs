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
use crate::Pallet as Thea;
use frame_benchmarking::v1::benchmarks;
use frame_support::traits::fungible::{hold::Mutate as HoldMutate, Mutate};
use frame_system::RawOrigin;
use parity_scale_codec::Decode;
use polkadex_primitives::UNIT_BALANCE;
use sp_std::collections::{btree_map::BTreeMap, btree_set::BTreeSet};
use thea_primitives::types::{
	IncomingMessage, MisbehaviourReport, SignedMessage, THEA_HOLD_REASON,
};
use thea_primitives::types::AssetMetadata;
use thea_primitives::TheaBenchmarkHelper;
use polkadex_primitives::AssetId;

fn generate_deposit_payload<T: Config>() -> Vec<Deposit<T::AccountId>> {
	sp_std::vec![Deposit {
		id: H256::zero().0.to_vec(),
		recipient: T::AccountId::decode(&mut &[0u8; 32][..]).unwrap(),
		asset_id: 1,
		amount: 0,
		extra: Vec::new(),
	}]
}

benchmarks! {
	submit_incoming_message {
		let b in 0 .. 256; // keep withing u8 range
		let message = Message {
			block_no: u64::MAX,
			nonce: 1,
			data: generate_deposit_payload::<T>().encode(),
			network: 0u8,
			payload_type: PayloadType::L1Deposit
		};
		let relayer: T::AccountId = T::AccountId::decode(&mut &[0u8; 32][..]).unwrap();
		<AllowListTestingRelayers<T>>::insert(0u8, relayer.clone());
		<T as pallet::Config>::NativeCurrency::mint_into(&relayer, (100000*UNIT_BALANCE).saturated_into()).unwrap();
	}: _(RawOrigin::Signed(relayer), message, 10000*UNIT_BALANCE)
	verify {
		// Nonce is updated only after execute_at number of blocks
		assert_eq!(<IncomingNonce::<T>>::get(0),0);
		assert_eq!(<IncomingMessages::<T>>::iter().count(), 0);
	}

	send_thea_message {
		let b in 0 .. 256; // keep within u8 bounds
		let network = b as u8;
		let data = [b as u8; 1_048_576].to_vec(); // 10MB
	}: _(RawOrigin::Root, data, network)
	verify {
		assert_eq!(<OutgoingNonce::<T>>::get(network), 1);
		assert_eq!(<OutgoingMessages::<T>>::iter().count(), 1);
	}

	update_incoming_nonce {
		let b in 1 .. u32::MAX;
		let network = 0;
		let nonce: u64 = b.into();
	}: _(RawOrigin::Root, nonce, network)
	verify {
		assert_eq!(<IncomingNonce::<T>>::get(network), nonce);
	}

	update_outgoing_nonce {
		let b in 1 .. u32::MAX;
		let network = 0;
		let nonce: u64 = b.into();
	}: _(RawOrigin::Root, nonce, network)
	verify {
		assert_eq!(<OutgoingNonce::<T>>::get(network), nonce);
	}

	add_thea_network {
		let network: u8 = 2;
	}: _(RawOrigin::Root, network, false, 20, 100*UNIT_BALANCE, 1000*UNIT_BALANCE)
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

	submit_signed_outgoing_messages {
		// Add OutgoinMessage
		let message = Message {
			block_no: u64::MAX,
			nonce: 1,
			data: generate_deposit_payload::<T>().encode(),
			network: 0u8,
			payload_type: PayloadType::L1Deposit
		};
		let network_id: u8 = 2;
		let nonce: u64 = 0;
		<OutgoingMessages<T>>::insert(network_id, nonce, message.clone());
		let mut signatures_map: BTreeMap<u32, T::Signature> = BTreeMap::new();
		let signature: T::Signature = sp_core::ecdsa::Signature::default().into();
		signatures_map.insert(0, signature.clone());
		let signed_message = SignedMessage {
			validator_set_id: 0,
			message: message,
			signatures: signatures_map
		};
		<SignedOutgoingMessages<T>>::insert(network_id, nonce, signed_message);
		let signatures = (network_id, nonce, signature);
		let sig_vec = vec![signatures];
	}: _(RawOrigin::None, 1, 0, sig_vec)
	verify {
		let signed_outgoing_message = <SignedOutgoingMessages<T>>::get(network_id, nonce).unwrap();
		assert!(signed_outgoing_message.signatures.len() == 2);
	}

	report_misbehaviour {
		// Create fisherman account with some balance
		let fisherman: T::AccountId = T::AccountId::decode(&mut &[0u8; 32][..]).unwrap();
		<T as pallet::Config>::NativeCurrency::mint_into(&fisherman, (100000*UNIT_BALANCE).saturated_into()).unwrap();
		let network_id: u8 = 2;
		let nonce: u64 = 0;
		let message = Message {
			block_no: u64::MAX,
			nonce: 1,
			data: generate_deposit_payload::<T>().encode(),
			network: 0u8,
			payload_type: PayloadType::L1Deposit
		};
		let incoming_message = IncomingMessage {
			message: message,
			relayer: fisherman.clone(),
			stake: (1000*UNIT_BALANCE).saturated_into(),
			execute_at: 1000
		};
		<IncomingMessagesQueue<T>>::insert(network_id, nonce, incoming_message);
	}: _(RawOrigin::Signed(fisherman), network_id, nonce)
	verify {
		let misbehaviour_report = <MisbehaviourReports<T>>::get(network_id, nonce);
		assert!(misbehaviour_report.is_some());
	}

	handle_misbehaviour {
		// Add MisbehaviourReports
		let relayer: T::AccountId = T::AccountId::decode(&mut &[0u8; 32][..]).unwrap();
		<T as pallet::Config>::NativeCurrency::mint_into(&relayer, (100000*UNIT_BALANCE).saturated_into()).unwrap();
		let fisherman: T::AccountId = T::AccountId::decode(&mut &[1u8; 32][..]).unwrap();
		<T as pallet::Config>::NativeCurrency::mint_into(&fisherman, (100000*UNIT_BALANCE).saturated_into()).unwrap();
		let relayer_stake_amount = 1 * UNIT_BALANCE;
		let fisherman_stake_amount = 1 * UNIT_BALANCE;
		T::NativeCurrency::hold(
				&THEA_HOLD_REASON,
				&relayer,
				relayer_stake_amount.saturated_into(),
			)?;
		T::NativeCurrency::hold(
				&THEA_HOLD_REASON,
				&fisherman,
				fisherman_stake_amount.saturated_into(),
			)?;
		let message = Message {
			block_no: u64::MAX,
			nonce: 0,
			data: generate_deposit_payload::<T>().encode(),
			network: 2u8,
			payload_type: PayloadType::L1Deposit
		};
		let incoming_message = IncomingMessage {
			message: message,
			relayer: relayer,
			stake: relayer_stake_amount,
			execute_at: 1000
		};
		let report = MisbehaviourReport {
			reported_msg: incoming_message,
			fisherman: fisherman,
			stake: fisherman_stake_amount
		};
		<MisbehaviourReports<T>>::insert(2, 0, report);
	}: _(RawOrigin::Root, 2, 0, true)

	on_initialize {
		let x in 1 .. 1_000;
		let network_len: usize = x as usize;
		let network_len: u8 = network_len as u8;
		// Update active network
		let mut networks: BTreeSet<u8> = BTreeSet::new();
		for i in 0..network_len {
			networks.insert(i);
		}
		<ActiveNetworks<T>>::put(networks.clone());
		T::TheaBenchmarkHelper::set_metadata(AssetId::Asset(1));
		let nonce = 1;
		for network in networks.iter() {
			let message = Message {
			block_no: 1,
			nonce: 1,
			data: generate_deposit_payload::<T>().encode(),
			network: *network,
			payload_type: PayloadType::L1Deposit
		};
		let incoming_message = IncomingMessage {
			message: message,
			relayer: T::AccountId::decode(&mut &[0u8; 32][..]).unwrap(),
			stake: (1000*UNIT_BALANCE).saturated_into(),
			execute_at: 0
		};
			<IncomingNonce<T>>::insert(*network, nonce);
			<IncomingMessagesQueue<T>>::insert(*network, nonce + 1, incoming_message.clone());
		}
	}: {
			<Thea<T>>::on_initialize((x as u32).into());
	} verify {
		for network in networks.iter() {
			let message = <IncomingMessages<T>>::get(*network, nonce);
			assert!(message.is_some());
		}
	}
}

#[cfg(test)]
use frame_benchmarking::impl_benchmark_test_suite;
use sp_core::H256;
use thea_primitives::types::Deposit;

#[cfg(test)]
impl_benchmark_test_suite!(Thea, crate::mock::new_test_ext(), crate::mock::Test);
