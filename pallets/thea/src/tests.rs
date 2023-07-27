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

fn any_signature() -> <Test as Config>::Signature {
	<Test as Config>::Signature::decode(&mut [1u8; 65].as_ref()).unwrap()
}

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

fn message_for_nonce(nonce: u64) -> Message {
	Message {
		block_no: u64::MAX,
		nonce,
		data: [255u8; 576].into(), //10 MB
		network: 0u8,
		is_key_change: false,
		validator_set_id: 0,
	}
}

use crate::ecdsa::AuthorityPair as Pair;
use frame_support::traits::OneSessionHandler;
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
		assert!(Thea::outgoing_nonce(1) == 1);
	})
}

#[test]
fn test_incoming_messages_bad_inputs() {
	new_test_ext().execute_with(|| {
		// set authorities
		let auth = set_200_validators();
		// bad origin (root)
		assert_err!(
			Thea::incoming_message(
				RuntimeOrigin::root(),
				message_for_nonce(1),
				vec![(0, any_signature())]
			),
			BadOrigin
		);
		// bad origin (some one signed)
		let message = message_for_nonce(1);
		let proper_sig = auth[0].sign(&message.encode());
		assert_err!(
			Thea::incoming_message(
				RuntimeOrigin::signed(1),
				message.clone(),
				vec![(0, proper_sig.clone())]
			),
			BadOrigin
		);
		// bad threshold
		assert_err!(
			Thea::validate_incoming_message(&message.clone(), &vec![(0, proper_sig.clone())]),
			InvalidTransaction::Custom(4)
		);

		// bad nonce (too big)
		assert_err!(
			Thea::validate_incoming_message(
				&message_for_nonce(u64::MAX),
				&vec![(0, proper_sig.clone())]
			),
			InvalidTransaction::Custom(1)
		);
		// bad nonce (too small)
		assert_err!(
			Thea::validate_incoming_message(
				&message_for_nonce(u64::MIN),
				&vec![(0, proper_sig.clone())]
			),
			InvalidTransaction::Custom(1)
		);
		// bad payload
		let mut bad_message = message.clone();
		bad_message.block_no = 1; // changing bit
		let bad_message_call = Call::<Test>::incoming_message {
			payload: bad_message,
			signatures: vec![(0, proper_sig.clone())],
		};
		assert!(Thea::validate_unsigned(TransactionSource::Local, &bad_message_call).is_err());
		// bad signature
		let bad_sig_call = Call::<Test>::incoming_message {
			payload: message.clone(),
			signatures: vec![(0, any_signature())],
		};
		assert!(Thea::validate_unsigned(TransactionSource::Local, &bad_sig_call).is_err());
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
		assert_err!(Thea::add_thea_network(RuntimeOrigin::none(), 1), BadOrigin);
		assert_err!(Thea::add_thea_network(RuntimeOrigin::signed(1), 1), BadOrigin);
		// add max number of networks
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(RuntimeOrigin::root(), net));
			let an = <ActiveNetworks<Test>>::get();
			assert_eq!(an.len(), net as usize + 1);
			assert!(an.get(&net).is_some());
		}
		// no failures on adding same network again
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(RuntimeOrigin::root(), net));
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
			assert_ok!(Thea::add_thea_network(RuntimeOrigin::root(), net));
			assert_ok!(Thea::remove_thea_network(RuntimeOrigin::root(), net));
			let an = <ActiveNetworks<Test>>::get();
			assert_eq!(an.len(), 0);
		}
		// populating everything
		for net in 0u8..=u8::MAX {
			assert_ok!(Thea::add_thea_network(RuntimeOrigin::root(), net));
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
