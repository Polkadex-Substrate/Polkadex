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
use parity_scale_codec::Encode;
use sp_core::Pair;
use sp_runtime::traits::BadOrigin;
use std::collections::BTreeMap;
use thea::ecdsa::AuthoritySignature;

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
			payload_type: PayloadType::L1Deposit,
		};
		let pair = sp_core::ecdsa::Pair::generate().0;

		<ValidatorSetId<Test>>::put(0);
		<Authorities<Test>>::insert(0, BoundedVec::truncate_from(vec![pair.public().into()]));
		let msg_prehashed = sp_io::hashing::sha2_256(&message.encode());
		let signature = pair.sign(&msg_prehashed);

		let signed_message = SignedMessage::new(message, 0, 0, signature.into());
		// bad origins
		assert_noop!(
			TheaHandler::incoming_message(RuntimeOrigin::root(), signed_message.clone()),
			BadOrigin
		);
		assert_noop!(
			TheaHandler::incoming_message(RuntimeOrigin::signed(1), signed_message.clone()),
			BadOrigin
		);
		// root
		assert_ok!(TheaHandler::incoming_message(RuntimeOrigin::none(), signed_message.clone()));
		assert_eq!(<ValidatorSetId<Test>>::get(), 0);
		// bad signature in unsigned verification
		let mut direct = signed_message.clone();
		direct.validator_set_id = 100;
		let bad_signature_call = Call::<Test>::incoming_message { payload: direct.clone() };
		assert!(
			TheaHandler::validate_unsigned(TransactionSource::Local, &bad_signature_call).is_err()
		);
		// bad message in unsigned verification
		let bad_message_call = Call::<Test>::incoming_message {
			payload: direct.clone(), // proper message
		};
		assert!(
			TheaHandler::validate_unsigned(TransactionSource::Local, &bad_message_call).is_err()
		);
		// bad nonce
		let mut vs = signed_message.clone();
		vs.message.nonce = 3;
		assert_noop!(
			TheaHandler::validate_incoming_message(&vs.clone(),),
			InvalidTransaction::Custom(1)
		);
		vs.message.nonce = 2;
		vs.validator_set_id = 1;
		assert_eq!(<ValidatorSetId<Test>>::get(), 0);
		<Authorities<Test>>::insert(1, BoundedVec::truncate_from(vec![pair.public().into(); 200]));
		assert_noop!(
			TheaHandler::validate_incoming_message(&vs.clone(),),
			InvalidTransaction::Custom(2)
		);
		<Authorities<Test>>::insert(1, BoundedVec::truncate_from(vec![pair.public().into()]));
		// invalid validator set id
		assert_noop!(
			TheaHandler::validate_incoming_message(&vs.clone(),),
			InvalidTransaction::Custom(4)
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
fn test_unsigned_call_validation() {
	new_test_ext().execute_with(|| {
		let pair = sp_core::ecdsa::Pair::generate().0;
		let public = <Test as Config>::TheaId::from(pair.public());

		assert_ok!(TheaHandler::insert_authorities(
			RuntimeOrigin::root(),
			BoundedVec::truncate_from(vec![public]),
			0
		));
		<ValidatorSetId<Test>>::put(0);

		let message = Message {
			block_no: 11,
			nonce: 1,
			data: vec![18, 52, 80],
			network: 1,
			payload_type: PayloadType::L1Deposit,
		};
		let encoded_payload = sp_io::hashing::sha2_256(&message.encode());
		let signature = pair.sign(&encoded_payload);
		let signed_message = SignedMessage::new(message, 0, 0, signature.into());
		println!("Running the validation..");
		let call = Call::<Test>::incoming_message { payload: signed_message };
		TheaHandler::validate_unsigned(TransactionSource::Local, &call).unwrap();
	})
}

#[test]
fn test_incoming_message_validator_change_payload() {
	new_test_ext().execute_with(|| {
		//Create SignedPayload
		let validator_set =
			ValidatorSet { set_id: 1, validators: vec![sp_core::ecdsa::Public::from_raw([1; 33])] };
		let network_id = 2;
		let message = Message {
			block_no: 10,
			nonce: 1,
			data: validator_set.encode(),
			network: network_id,
			payload_type: PayloadType::ScheduledRotateValidators,
		};
		let sign = sp_core::ecdsa::Signature::default().into();
		let mut signature_map = BTreeMap::new();
		signature_map.insert(0, sign);
		let signed_message_sv =
			SignedMessage { validator_set_id: 0, message, signatures: signature_map };
		assert_ok!(TheaHandler::incoming_message(RuntimeOrigin::none(), signed_message_sv.clone()));
		let authorities = <Authorities<Test>>::get(1);
		assert_eq!(authorities.len(), 1);
		assert_eq!(authorities[0], sp_core::ecdsa::Public::from_raw([1; 33]).into());
		let validator_rotated_message = Message {
			block_no: 0,
			nonce: 1,
			data: vec![1, 2, 3, 4, 5],
			network: network_id,
			payload_type: PayloadType::ValidatorsRotated,
		};
		let sign = sp_core::ecdsa::Signature::default().into();
		let mut signature_map = BTreeMap::new();
		signature_map.insert(0, sign);
		let signed_message = SignedMessage {
			validator_set_id: 0,
			message: validator_rotated_message,
			signatures: signature_map,
		};
		assert_ok!(TheaHandler::incoming_message(RuntimeOrigin::none(), signed_message.clone()));
		assert_eq!(<ValidatorSetId<Test>>::get(), 1);
		assert_noop!(
			TheaHandler::incoming_message(RuntimeOrigin::none(), signed_message_sv.clone()),
			Error::<Test>::InvalidValidatorSetId
		);
	})
}
