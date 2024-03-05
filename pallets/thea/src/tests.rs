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

//! Tests for "thea" pallet.

use crate::{mock::*, *};
use frame_support::{assert_err, assert_ok};
use sp_core::Pair as CorePair;
use sp_runtime::DispatchError::BadOrigin;
const WELL_KNOWN: &str = "bottom drive obey lake curtain smoke basket hold race lonely fit walk";
use sp_std::collections::btree_set::BTreeSet;

static PAYLOAD: [u8; 10_485_760] = [u8::MAX; 10_485_760];

fn set_200_validators() -> [Pair; 200] {
	let mut validators = Vec::with_capacity(200);
	for i in 0..200 {
		validators
			.push(Pair::generate_with_phrase(Some(format!("{}//{}", WELL_KNOWN, i).as_str())).0);
	}
	let mut bv: BoundedVec<<Test as Config>::TheaId, <Test as Config>::MaxAuthorities> =
		BoundedVec::with_max_capacity();
	validators.clone().into_iter().for_each(|v| bv.try_push(v.public()).unwrap());
	<Authorities<Test>>::insert(0, bv);
	validators
		.try_into()
		.unwrap_or_else(|_| panic!("Could not convert validators to array"))
}

use crate::ecdsa::AuthorityPair as Pair;
use frame_support::traits::OneSessionHandler;
use polkadex_primitives::UNIT_BALANCE;

#[test]
fn test_session_change() {
	new_test_ext().execute_with(|| {
		let mut authorities: Vec<(&u64, <Test as Config>::TheaId)> = Vec::with_capacity(200);
		for i in 0..200u64 {
			authorities.push((
				&1,
				Pair::generate_with_phrase(Some(format!("{}//{}", WELL_KNOWN, i).as_str()))
					.0
					.public()
					.into(),
			));
		}

		let mut queued: Vec<(&u64, <Test as Config>::TheaId)> = Vec::with_capacity(200);
		for i in 0..200u64 {
			queued.push((
				&1,
				Pair::generate_with_phrase(Some(format!("{}//{}", WELL_KNOWN, i).as_str()))
					.0
					.public()
					.into(),
			));
		}

		let mut networks = BTreeSet::new();
		networks.insert(1);
		<ActiveNetworks<Test>>::put(networks);
		assert!(Thea::validator_set_id() == 0);
		assert!(Thea::outgoing_nonce(1) == 0);
		let current_authorities: Vec<<Test as Config>::TheaId> =
			authorities.iter().map(|(_, public)| public.clone()).collect();
		<ValidatorSetId<Test>>::put(0);
		<Authorities<Test>>::insert(0, BoundedVec::truncate_from(current_authorities));
		// Simulating the on_new_session to last epoch of an era.
		Thea::on_new_session(false, authorities.into_iter(), queued.clone().into_iter());
		assert!(Thea::validator_set_id() == 0);
		assert!(Thea::outgoing_nonce(1) == 1); // Thea validator session change message is generated here

		let message = Thea::get_outgoing_messages(1, 1).unwrap();
		assert_eq!(message.nonce, 1);
		let validator_set: ValidatorSet<<Test as Config>::TheaId> =
			ValidatorSet::decode(&mut &message.data[..]).unwrap();
		let queued_validators: Vec<<Test as Config>::TheaId> =
			queued.iter().map(|(_, public)| public.clone()).collect();
		assert_eq!(validator_set.set_id, 1);
		assert_eq!(validator_set.validators, queued_validators);

		// Simulating the on_new_session to the first epoch of the next era.
		Thea::on_new_session(false, queued.clone().into_iter(), queued.clone().into_iter());
		assert!(Thea::validator_set_id() == 1);
		assert!(Thea::outgoing_nonce(1) == 2);
		let message = Thea::get_outgoing_messages(1, 2).unwrap();
		assert_eq!(message.nonce, 2);
		assert!(message.data.is_empty());
	})
}

#[test]
fn test_send_thea_message_proper_inputs() {
	new_test_ext().execute_with(|| {
		// each 25%th of all possible networks
		for n in (0u8..=u8::MAX).step_by((u8::MAX / 4).into()) {
			set_200_validators(); // setting max validators
			assert_ok!(Thea::send_thea_message(
				RuntimeOrigin::root(),
				// 10MB of u8::MAX payload
				PAYLOAD.to_vec(),
				n
			));
		}
	})
}

#[test]
fn test_send_thea_message_bad_inputs() {
	new_test_ext().execute_with(|| {
		// bad origin
		assert_err!(Thea::send_thea_message(RuntimeOrigin::none(), vec!(), 0), BadOrigin);
		assert_err!(Thea::send_thea_message(RuntimeOrigin::signed(0), vec!(), 0), BadOrigin);
		assert_err!(Thea::send_thea_message(RuntimeOrigin::signed(1), vec!(), 0), BadOrigin);
		assert_err!(
			Thea::send_thea_message(RuntimeOrigin::signed(u32::MAX.into()), vec!(), 0),
			BadOrigin
		);
		assert_err!(Thea::send_thea_message(RuntimeOrigin::signed(u64::MAX), vec!(), 0), BadOrigin);
		assert_eq!(<OutgoingNonce<Test>>::get(0), 0);
		assert_eq!(<OutgoingMessages<Test>>::get(0, 1), None);
	})
}

#[test]
fn test_update_incoming_nonce_all() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_err!(Thea::update_incoming_nonce(RuntimeOrigin::none(), u64::MAX, 0), BadOrigin);
		assert_err!(Thea::update_incoming_nonce(RuntimeOrigin::signed(1), u64::MAX, 0), BadOrigin);
		assert_err!(
			Thea::update_incoming_nonce(RuntimeOrigin::signed(u32::MAX.into()), u64::MAX, 0),
			BadOrigin
		);
		assert_err!(
			Thea::update_incoming_nonce(RuntimeOrigin::signed(u64::MAX), u64::MAX, 0),
			BadOrigin
		);
		// proper cases
		<IncomingNonce<Test>>::set(0, 0);
		assert_ok!(Thea::update_incoming_nonce(RuntimeOrigin::root(), 10, 0));
		assert_ok!(Thea::update_incoming_nonce(RuntimeOrigin::root(), 100, 0));
		assert_ok!(Thea::update_incoming_nonce(RuntimeOrigin::root(), 10_000, 0));
		assert_ok!(Thea::update_incoming_nonce(RuntimeOrigin::root(), u32::MAX.into(), 0));
		assert_ok!(Thea::update_incoming_nonce(RuntimeOrigin::root(), u64::MAX, 0));
	})
}

#[test]
fn test_update_outgoing_nonce_all() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_err!(Thea::update_outgoing_nonce(RuntimeOrigin::none(), u64::MAX, 0), BadOrigin);
		assert_err!(Thea::update_outgoing_nonce(RuntimeOrigin::signed(1), u64::MAX, 0), BadOrigin);
		assert_err!(
			Thea::update_outgoing_nonce(RuntimeOrigin::signed(u32::MAX.into()), u64::MAX, 0),
			BadOrigin
		);
		assert_err!(
			Thea::update_outgoing_nonce(RuntimeOrigin::signed(u64::MAX), u64::MAX, 0),
			BadOrigin
		);

		// proper cases
		<IncomingNonce<Test>>::set(0, 0);
		assert_ok!(Thea::update_outgoing_nonce(RuntimeOrigin::root(), 10, 0));
		assert_ok!(Thea::update_outgoing_nonce(RuntimeOrigin::root(), 100, 0));
		assert_ok!(Thea::update_outgoing_nonce(RuntimeOrigin::root(), 10_000, 0));
		assert_ok!(Thea::update_outgoing_nonce(RuntimeOrigin::root(), u32::MAX.into(), 0));
		assert_ok!(Thea::update_outgoing_nonce(RuntimeOrigin::root(), u64::MAX, 0));
	})
}

#[test]
fn test_add_thea_network_full() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_err!(
			Thea::add_thea_network(
				RuntimeOrigin::none(),
				1,
				false,
				20,
				100 * UNIT_BALANCE,
				1000 * UNIT_BALANCE
			),
			BadOrigin
		);
		assert_err!(
			Thea::add_thea_network(
				RuntimeOrigin::signed(1),
				1,
				false,
				20,
				100 * UNIT_BALANCE,
				1000 * UNIT_BALANCE
			),
			BadOrigin
		);
		// add max number of networks
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(
				RuntimeOrigin::root(),
				net,
				false,
				20,
				100 * UNIT_BALANCE,
				1000 * UNIT_BALANCE
			));
			let an = <ActiveNetworks<Test>>::get();
			assert_eq!(an.len(), net as usize + 1);
			assert!(an.get(&net).is_some());
		}
		// no failures on adding same network again
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(
				RuntimeOrigin::root(),
				net,
				false,
				20,
				100 * UNIT_BALANCE,
				1000 * UNIT_BALANCE
			));
		}
	})
}

#[test]
fn test_remove_thea_network_full() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_err!(Thea::remove_thea_network(RuntimeOrigin::none(), 1), BadOrigin);
		assert_err!(Thea::remove_thea_network(RuntimeOrigin::signed(1), 1), BadOrigin);
		// make sure it's not blowing on absent network
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::remove_thea_network(RuntimeOrigin::root(), net));
		}
		// add one and remove one
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(
				RuntimeOrigin::root(),
				net,
				false,
				20,
				100 * UNIT_BALANCE,
				1000 * UNIT_BALANCE
			));
			assert_ok!(Thea::remove_thea_network(RuntimeOrigin::root(), net));
			let an = <ActiveNetworks<Test>>::get();
			assert_eq!(an.len(), 0);
		}
		// populating everything
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(
				RuntimeOrigin::root(),
				net,
				false,
				20,
				100 * UNIT_BALANCE,
				1000 * UNIT_BALANCE
			));
		}
		// remove reverse order
		for net in (0u8..=u8::MAX).rev() {
			assert_ok!(Thea::remove_thea_network(RuntimeOrigin::root(), net));
			let an = <ActiveNetworks<Test>>::get();
			// when we remove one it should be exact same len as value :)
			assert_eq!(an.len(), net as usize);
			assert!(an.get(&net).is_none());
		}
	})
}

use frame_support::traits::Currency;

#[test]
fn test_report_misbehaviour_happy_path() {
	new_test_ext().execute_with(|| {
		// Add messgae to IncomingMessagesQueue storage
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce: 1,
			network: 1,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let config = thea_primitives::types::NetworkConfig {
			fork_period: 0,
			min_stake: 1_000_000,
			fisherman_stake: 1_000_000,
			network_type: NetworkType::Parachain,
		};
		<NetworkConfig<Test>>::insert(network, config);
		let relayer = 1u64;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 10000000000000000000000);
		let fisherman = 2u64;
		let _ = Balances::deposit_creating(&fisherman, 10000000000000000000000);
		let stake = 1000000000000000000000;
		let incoming_message =
			thea_primitives::types::IncomingMessage { message, relayer, stake, execute_at: 0 };
		<IncomingMessagesQueue<Test>>::insert(network, 1, incoming_message);
		// Report Misbehaviour
		assert_ok!(Thea::report_misbehaviour(RuntimeOrigin::signed(fisherman), network, 1));
	})
}

use frame_support::{
	assert_noop,
	traits::{fungible::MutateHold, tokens::Precision},
};
use thea_primitives::types::{AssetMetadata, IncomingMessage, SignedMessage, THEA_HOLD_REASON};

#[test]
fn test_report_misbehaviour_not_enough_stake() {
	new_test_ext().execute_with(|| {
		// Add messgae to IncomingMessagesQueue storage
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce: 1,
			network: 1,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let config = thea_primitives::types::NetworkConfig {
			fork_period: 0,
			min_stake: 1_000_000_000_000_000_000_000_000_000,
			fisherman_stake: 1_000_000_000_000_000_000_000_000,
			network_type: NetworkType::Parachain,
		};
		<NetworkConfig<Test>>::insert(network, config);
		let relayer = 1u64;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 10000000000000000000000);
		let fisherman = 2u64;
		let _ = Balances::deposit_creating(&fisherman, 10000000000000000000000);
		let stake = 1000000000000000000000;
		let incoming_message =
			thea_primitives::types::IncomingMessage { message, relayer, stake, execute_at: 0 };
		<IncomingMessagesQueue<Test>>::insert(network, 1, incoming_message);
		// Report Misbehaviour
		assert_noop!(
			Thea::report_misbehaviour(RuntimeOrigin::signed(fisherman), network, 1),
			Error::<Test>::NotEnoughStake
		);
	})
}

#[test]
fn test_handle_misbehaviour_happy_path_valid_proposal() {
	new_test_ext().execute_with(|| {
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce: 1,
			network: 1,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let config = thea_primitives::types::NetworkConfig {
			fork_period: 0,
			min_stake: 1_000_000,
			fisherman_stake: 1_000_000,
			network_type: NetworkType::Parachain,
		};
		<NetworkConfig<Test>>::insert(network, config);
		let relayer = 1u64;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 10000000000000000000000);
		let fisherman = 2u64;
		let _ = Balances::deposit_creating(&fisherman, 10000000000000000000000);
		let stake = 1000000000000000000000;
		let incoming_message =
			thea_primitives::types::IncomingMessage { message, relayer, stake, execute_at: 0 };
		<IncomingMessagesQueue<Test>>::insert(network, 1, incoming_message);
		// Report Misbehaviour
		assert_ok!(Thea::report_misbehaviour(RuntimeOrigin::signed(fisherman), network, 1));
		assert_ok!(Thea::handle_misbehaviour(RuntimeOrigin::root(), network, 1, true));
		// Check Balance
		assert_eq!(Balances::free_balance(&relayer), 9000000000000000000000);
		assert_eq!(Balances::free_balance(&fisherman), 11000000000000000000000);
	})
}

#[test]
fn test_handle_misbehaviour_happy_path_invalid_proposal() {
	new_test_ext().execute_with(|| {
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce: 1,
			network: 1,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let config = thea_primitives::types::NetworkConfig {
			fork_period: 0,
			min_stake: 1_000_000,
			fisherman_stake: 1_000_000,
			network_type: NetworkType::Parachain,
		};
		<NetworkConfig<Test>>::insert(network, config);
		let relayer = 1u64;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 10000000000000000000000);
		let fisherman = 2u64;
		let _ = Balances::deposit_creating(&fisherman, 10000000000000000000000);
		let stake = 1000000000000000000000;
		let incoming_message =
			thea_primitives::types::IncomingMessage { message, relayer, stake, execute_at: 0 };
		<IncomingMessagesQueue<Test>>::insert(network, 1, incoming_message);
		// Report Misbehaviour
		assert_ok!(Thea::report_misbehaviour(RuntimeOrigin::signed(fisherman), network, 1));
		assert_ok!(Thea::handle_misbehaviour(RuntimeOrigin::root(), network, 1, false));
		// Check Balance
		assert_eq!(Balances::free_balance(&relayer), 10000000000000000000000);
		assert_eq!(Balances::free_balance(&fisherman), 9999999999999998000000);
	})
}

#[test]
fn test_submit_signed_outgoing_messages_happy_path() {
	new_test_ext().execute_with(|| {
		// Insert OutgoingMessages Storage
		let network = 2;
		let nonce = 1;
		let validator_set_id = 1;
		let auth_index = 0;
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		<OutgoingMessages<Test>>::insert(network, nonce, message);
		let signature = sp_core::ecdsa::Signature::default().into();
		let signatures = vec![(network, nonce, signature)];
		assert_ok!(Thea::submit_signed_outgoing_messages(
			RuntimeOrigin::none(),
			auth_index,
			validator_set_id,
			signatures.clone()
		));
		assert!(<SignedOutgoingMessages<Test>>::get(network, nonce).is_some());
		let mut auth = <Authorities<Test>>::get(validator_set_id);
		auth.try_push(sp_core::ecdsa::Public::from_raw([1; 33]).into()).unwrap();
		auth.try_push(sp_core::ecdsa::Public::from_raw([2; 33]).into()).unwrap();
		let auth_index = 2;
		assert!(<SignedOutgoingNonce<Test>>::get(network) == 0);
		assert_ok!(Thea::submit_signed_outgoing_messages(
			RuntimeOrigin::none(),
			auth_index,
			validator_set_id,
			signatures
		));
		assert!(<SignedOutgoingNonce<Test>>::get(network) == 1);
	})
}

#[test]
fn test_submit_signed_outgoing_messages_message_not_found() {
	new_test_ext().execute_with(|| {
		let network = 2;
		let nonce = 1;
		let validator_set_id = 1;
		let auth_index = 0;
		let _message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let signature = sp_core::ecdsa::Signature::default().into();
		let signatures = vec![(network, nonce, signature)];
		assert_noop!(
			Thea::submit_signed_outgoing_messages(
				RuntimeOrigin::none(),
				auth_index,
				validator_set_id,
				signatures.clone()
			),
			Error::<Test>::MessageNotFound
		);
	})
}

#[test]
fn test_on_initialize_happy_path() {
	new_test_ext().execute_with(|| {
		// Insert in Active Networks
		let mut networks: BTreeSet<Network> = BTreeSet::new();
		let network = 1;
		networks.insert(network);
		<ActiveNetworks<Test>>::put(networks);
		// Update next Nonce
		let nonce = 0;
		<IncomingNonce<Test>>::insert(network, nonce);
		let relayer = 1u64;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 100 * UNIT_BALANCE);
		let stake = 1 * UNIT_BALANCE;
		// Reserve balance
		Balances::hold(&THEA_HOLD_REASON, &relayer, stake).unwrap();
		// Add message to IncomingMessagesQueue
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let incoming_message = IncomingMessage { message, relayer, stake, execute_at: 0 };
		<IncomingMessagesQueue<Test>>::insert(network, nonce.saturating_add(1), incoming_message);
		Thea::on_initialize(1);
		assert_eq!(<IncomingNonce<Test>>::get(network), 1);
		assert_eq!(Balances::free_balance(&relayer), 100 * UNIT_BALANCE);
	})
}

#[test]
fn test_validate_signed_outgoing_message_happy_path() {
	new_test_ext().execute_with(|| {
		let validator = sp_core::ecdsa::Pair::from_seed(b"12345678901234567890123456789012");
		let validator_set_id = 1;
		let mut auths = <Authorities<Test>>::get(validator_set_id);
		auths.try_push(validator.public().into()).unwrap();
		<Authorities<Test>>::insert(validator_set_id, auths);
		// Insert SignedOutgoingNonce
		let nonce = 1;
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());
		let signature = validator.sign_prehashed(&msg_hash);
		let signatures = vec![(network, nonce, signature.into())];
		<SignedOutgoingNonce<Test>>::insert(network, nonce.saturating_sub(1));
		<OutgoingMessages<Test>>::insert(network, nonce, message);
		assert_ok!(Thea::validate_signed_outgoing_message(&0, &validator_set_id, &signatures));
	})
}

#[test]
fn test_validate_signed_outgoing_message_custom_error_1() {
	new_test_ext().execute_with(|| {
		let validator = sp_core::ecdsa::Pair::from_seed(b"12345678901234567890123456789012");
		let validator_set_id = 1;
		let mut auths = <Authorities<Test>>::get(validator_set_id);
		auths.try_push(validator.public().into()).unwrap();
		<Authorities<Test>>::insert(validator_set_id, auths);
		// Insert SignedOutgoingNonce
		let nonce = 1;
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());
		let signature = validator.sign_prehashed(&msg_hash);
		let signatures = vec![(network, nonce, signature.into())];
		<SignedOutgoingNonce<Test>>::insert(network, nonce.saturating_sub(1));
		<OutgoingMessages<Test>>::insert(network, nonce, message);
		assert_noop!(
			Thea::validate_signed_outgoing_message(&10, &validator_set_id, &signatures),
			InvalidTransaction::Custom(1)
		);
	})
}

#[test]
fn test_validate_signed_outgoing_message_returns_custom_error() {
	new_test_ext().execute_with(|| {
		let validator = sp_core::ecdsa::Pair::from_seed(b"12345678901234567890123456789012");
		let validator_set_id = 1;
		let mut auths = <Authorities<Test>>::get(validator_set_id);
		auths.try_push(validator.public().into()).unwrap();
		<Authorities<Test>>::insert(validator_set_id, auths);
		// Insert SignedOutgoingNonce
		let nonce = 1;
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());
		let signature = validator.sign_prehashed(&msg_hash);
		let signatures = vec![(network, nonce, signature.into())];
		assert_noop!(
			Thea::validate_signed_outgoing_message(&0, &validator_set_id, &signatures),
			InvalidTransaction::Custom(3)
		);
		<SignedOutgoingNonce<Test>>::insert(network, 50);
		assert_noop!(
			Thea::validate_signed_outgoing_message(&0, &validator_set_id, &signatures),
			InvalidTransaction::Custom(2)
		);
	})
}

#[test]
fn test_validate_signed_outgoing_message_wrong_sig() {
	new_test_ext().execute_with(|| {
		let validator = sp_core::ecdsa::Pair::from_seed(b"12345678901234567890123456789012");
		let validator_set_id = 1;
		let mut auths = <Authorities<Test>>::get(validator_set_id);
		auths.try_push(validator.public().into()).unwrap();
		<Authorities<Test>>::insert(validator_set_id, auths);
		// Insert SignedOutgoingNonce
		let nonce = 1;
		let network = 2;
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let _ = sp_io::hashing::sha2_256(message.encode().as_slice());
		let signature = sp_core::ecdsa::Signature::default();
		let signatures = vec![(network, nonce, signature.into())];
		<SignedOutgoingNonce<Test>>::insert(network, nonce.saturating_sub(1));
		<OutgoingMessages<Test>>::insert(network, nonce, message);
		assert_noop!(
			Thea::validate_signed_outgoing_message(&0, &validator_set_id, &signatures),
			InvalidTransaction::Custom(6)
		);
	})
}

#[test]
fn test_submit_incoming_message_happy_path_first_message() {
	new_test_ext().execute_with(|| {
		let relayer = 1u64;
		let network_id = 2;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 100 * UNIT_BALANCE);
		let stake = 1 * UNIT_BALANCE;
		let message = Message {
			block_no: 0,
			nonce: 1,
			network: network_id,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let network_config = thea_primitives::types::NetworkConfig {
			fork_period: 0,
			min_stake: 1 * UNIT_BALANCE,
			fisherman_stake: 1 * UNIT_BALANCE,
			network_type: NetworkType::Parachain,
		};
		<NetworkConfig<Test>>::insert(network_id, network_config);
		<AllowListTestingRelayers<Test>>::insert(network_id, relayer);
		assert_ok!(Thea::submit_incoming_message(
			RuntimeOrigin::signed(relayer),
			message.clone(),
			stake
		));
		assert_eq!(Balances::reserved_balance(&relayer), 1 * UNIT_BALANCE);
		let relayer_2 = 2u64;
		let _ = Balances::deposit_creating(&relayer_2, 100 * UNIT_BALANCE);
		let message_two = Message {
			block_no: 0,
			nonce: 1,
			network: network_id,
			payload_type: PayloadType::L1Deposit,
			data: vec![1u8; 10],
		};
		let new_stake = 2 * UNIT_BALANCE;
		<AllowListTestingRelayers<Test>>::insert(network_id, relayer_2);
		assert_ok!(Thea::submit_incoming_message(
			RuntimeOrigin::signed(relayer_2),
			message_two.clone(),
			new_stake
		));
		assert_eq!(Balances::reserved_balance(&relayer_2), 2 * UNIT_BALANCE);
		assert_eq!(Balances::reserved_balance(&relayer), 0);
	})
}

#[test]
fn test_add_signature() {
	new_test_ext().execute_with(|| {
		let network = 2;
		let nonce = 1;
		let validator_set_id = 1;
		let auth_index = 0;
		let message = Message {
			block_no: 0,
			nonce,
			network,
			payload_type: PayloadType::L1Deposit,
			data: vec![],
		};
		let signature = sp_core::ecdsa::Signature::default();
		let mut signed_message =
			SignedMessage::new(message.clone(), validator_set_id, auth_index, signature.clone());
		assert_eq!(signed_message.signatures.len(), 1);
		assert_eq!(signed_message.signatures.get(&0).unwrap().clone(), signature);
		let new_validator_set_id = 2;
		let new_signature = sp_core::ecdsa::Signature::from_raw([1; 65]);
		signed_message.add_signature(
			message,
			new_validator_set_id,
			auth_index,
			new_signature.clone(),
		);
		assert_eq!(signed_message.signatures.len(), 1);
		assert_eq!(signed_message.signatures.get(&0).unwrap().clone(), new_signature);
	})
}

#[test]
fn test_asset_metadata_convert_from_native_decimals() {
	let metadata = AssetMetadata::new(6).unwrap();
	assert_eq!(
		metadata.convert_from_native_decimals(1000000000000000000000000),
		1000000000000000000
	);
}

#[test]
fn test_locks() {
	new_test_ext().execute_with(|| {
		let relayer = 1u64;
		// Mint Balance
		let _ = Balances::deposit_creating(&relayer, 100 * UNIT_BALANCE);
		let stake = 1 * UNIT_BALANCE;
		// Reserve balance
		Balances::hold(&THEA_HOLD_REASON, &relayer, stake).unwrap();
		Balances::hold(&THEA_HOLD_REASON, &relayer, stake).unwrap();
		assert_eq!(Balances::reserved_balance(&relayer), 2 * UNIT_BALANCE);
		Balances::release(&THEA_HOLD_REASON, &relayer, stake, Precision::BestEffort).unwrap();
		assert_eq!(Balances::reserved_balance(&relayer), 1 * UNIT_BALANCE);
		Balances::release(&THEA_HOLD_REASON, &relayer, stake, Precision::BestEffort).unwrap();
		assert_eq!(Balances::reserved_balance(&relayer), 0);
	})
}
