// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
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

use crate::{
	mock::{new_test_ext, Test, *},
	TransactionSource, *,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::EventRecord;
use parity_scale_codec::{Decode, Encode};
use sp_core::{ByteArray, Pair};
use sp_runtime::traits::BadOrigin;
use thea_primitives::ValidatorSet;

fn assert_last_event<T: crate::Config>(generic_event: <T as crate::Config>::RuntimeEvent) {
	let events = frame_system::Pallet::<T>::events();
	let system_event: <T as frame_system::Config>::RuntimeEvent = generic_event.into();
	// compare to the last event record
	let EventRecord { event, .. } = &events[events.len() - 1];
	assert_eq!(event, &system_event);
}

#[test]
fn test_insert_authorities_full() {
	new_test_ext().execute_with(|| {
		let authorities =
			BoundedVec::truncate_from(vec![sp_core::ecdsa::Pair::generate().0.public().into()]);
		// bad origins
		assert_noop!(
			TheaHandler::insert_authorities(RuntimeOrigin::none(), authorities.clone(), 0),
			BadOrigin
		);
		assert_noop!(
			TheaHandler::insert_authorities(RuntimeOrigin::signed(1), authorities.clone(), 0),
			BadOrigin
		);
		// proper case
		assert_ok!(TheaHandler::insert_authorities(
			RuntimeOrigin::root(),
			authorities.clone(),
			111
		));
		assert_eq!(<Authorities<Test>>::get(111), authorities);
		assert_eq!(<ValidatorSetId<Test>>::get(), 111);
	})
}

#[test]
fn test_incoming_message_full() {
	new_test_ext().execute_with(|| {
		let message = Message {
			block_no: 10,
			nonce: 1,
			data: vec![1, 2, 3, 4, 5],
			network: 1,
			is_key_change: false,
			validator_set_id: 0,
		};
		let pair = sp_core::ecdsa::Pair::generate().0;

		<ValidatorSetId<Test>>::put(0);
		<Authorities<Test>>::insert(0, BoundedVec::truncate_from(vec![pair.public().into()]));
		let msg_prehashed = sp_io::hashing::sha2_256(&message.encode());
		let signature = pair.sign(&msg_prehashed);
		// bad origins
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::root(),
				message.clone(),
				vec![(0, signature.clone().into())]
			),
			BadOrigin
		);
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::signed(1),
				message.clone(),
				vec![(0, signature.clone().into())]
			),
			BadOrigin
		);
		// root
		assert_ok!(TheaHandler::incoming_message(
			RuntimeOrigin::none(),
			message.clone(),
			vec![(0, signature.clone().into())]
		));
		assert_eq!(<ValidatorSetId<Test>>::get(), 0);
		// bad signature in unsigned verification
		let mut direct = message.clone();
		direct.validator_set_id = 100;
		let bad_signature_call = Call::<Test>::incoming_message {
			payload: direct.clone(),
			signatures: vec![(0, signature.clone().into())],
		};
		assert!(
			TheaHandler::validate_unsigned(TransactionSource::Local, &bad_signature_call).is_err()
		);
		// bad message in unsigned verification
		let bad_message_call = Call::<Test>::incoming_message {
			payload: direct.clone(), // proper message
			signatures: vec![(
				0,
				<Test as Config>::Signature::decode(&mut [0u8; 65].as_ref()).unwrap(),
			)],
		};
		assert!(
			TheaHandler::validate_unsigned(TransactionSource::Local, &bad_message_call).is_err()
		);
		// bad nonce
		let mut vs = message.clone();
		vs.nonce = 3;
		assert_noop!(
			TheaHandler::validate_incoming_message(
				&vs.clone(),
				&vec![(0, signature.clone().into())]
			),
			InvalidTransaction::Custom(1)
		);
		vs.nonce = 2;
		vs.validator_set_id = 1;
		assert_eq!(<ValidatorSetId<Test>>::get(), 0);
		// invalid validator set id
		assert_noop!(
			TheaHandler::validate_incoming_message(
				&vs.clone(),
				&vec![(0, signature.clone().into())]
			),
			InvalidTransaction::Custom(2)
		);
	})
}

#[test]
fn update_incoming_nonce_full() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_noop!(TheaHandler::update_incoming_nonce(RuntimeOrigin::none(), 1), BadOrigin);
		assert_noop!(TheaHandler::update_incoming_nonce(RuntimeOrigin::signed(1), 1), BadOrigin);
		// ok cases
		assert_ok!(TheaHandler::update_incoming_nonce(RuntimeOrigin::root(), 1));
		assert_eq!(1, <IncomingNonce<Test>>::get());
		assert_ok!(TheaHandler::update_incoming_nonce(RuntimeOrigin::root(), u64::MAX / 2));
		assert_eq!(u64::MAX / 2, <IncomingNonce<Test>>::get());
		assert_ok!(TheaHandler::update_incoming_nonce(RuntimeOrigin::root(), u64::MAX));
		assert_eq!(u64::MAX, <IncomingNonce<Test>>::get());
	})
}

#[test]
fn update_outgoing_nonce_full() {
	new_test_ext().execute_with(|| {
		// bad origins
		assert_noop!(TheaHandler::update_outgoing_nonce(RuntimeOrigin::none(), 1), BadOrigin);
		assert_noop!(TheaHandler::update_outgoing_nonce(RuntimeOrigin::signed(1), 1), BadOrigin);
		// ok cases
		assert_ok!(TheaHandler::update_outgoing_nonce(RuntimeOrigin::root(), 1));
		assert_eq!(1, <OutgoingNonce<Test>>::get());
		assert_ok!(TheaHandler::update_outgoing_nonce(RuntimeOrigin::root(), u64::MAX / 2));
		assert_eq!(u64::MAX / 2, <OutgoingNonce<Test>>::get());
		assert_ok!(TheaHandler::update_outgoing_nonce(RuntimeOrigin::root(), u64::MAX));
		assert_eq!(u64::MAX, <OutgoingNonce<Test>>::get());
	})
}

#[test]
fn real_test_vector() {
	new_test_ext().execute_with(|| {
		let public_bytes = hex::decode("020a1091341fe5664bfa1782d5e04779689068c916b04cb365ec3153755684d9a1").unwrap();
		let public = <Test as Config>::TheaId::from_slice(&public_bytes).unwrap();

		let signature_bytes = hex::decode("f665f69c959c4a3cbc54ec4de8a566f1897c648fe6c33ab1056ef11fcdd7ad937f4bae4540c18c1a4c61acc4a8bb8c11cafaafe8a06cfb7298e3f9ffba71d33500").unwrap();
		let signature = sp_core::ecdsa::Signature::from_slice(&signature_bytes).unwrap();

		<Authorities<Test>>::insert(0, BoundedVec::truncate_from(vec![public]));
		<ValidatorSetId<Test>>::put(0);

		let message = Message { block_no: 11, nonce: 1, data: vec![18, 52, 80], network: 1, is_key_change: false, validator_set_id: 0 };
		println!("Running the validation..");
		TheaHandler::validate_incoming_message(&message, &vec![(0, signature.into())]).unwrap();
	})
}
