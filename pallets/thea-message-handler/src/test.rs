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
	fixtures::{produce_authorities, M, M_KC, SIG},
	mock::{new_test_ext, Test, *},
	TransactionSource, *,
};
use frame_support::{assert_noop, assert_ok};
use frame_system::EventRecord;
use parity_scale_codec::{Decode, Encode};
use sp_runtime::traits::BadOrigin;
use thea_primitives::ValidatorSet;

fn get_valid_signature<T: Config>() -> T::Signature {
	<T as crate::Config>::Signature::decode(&mut SIG.as_ref()).unwrap()
}

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
		let authorities = produce_authorities::<Test>();
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
		// bad origins
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::root(),
				vec!(u128::MAX),
				M.clone(),
				get_valid_signature::<Test>()
			),
			BadOrigin
		);
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::signed(1),
				vec!(u128::MAX),
				M.clone(),
				get_valid_signature::<Test>()
			),
			BadOrigin
		);
		// root
		assert_ok!(TheaHandler::incoming_message(
			RuntimeOrigin::none(),
			vec!(u128::MAX),
			M.clone(),
			get_valid_signature::<Test>()
		));
		// bad signature in unsigned verification
		let mut direct = M.clone();
		direct.validator_set_len = 100;
		let bad_signature_call = Call::<Test>::incoming_message {
			bitmap: vec![u128::MAX],
			payload: direct,
			signature: get_valid_signature::<Test>(),
		};
		assert!(
			TheaHandler::validate_unsigned(TransactionSource::Local, &bad_signature_call).is_err()
		);
		// bad message in unsigned verification
		let bad_message_call = Call::<Test>::incoming_message {
			bitmap: vec![u128::MAX],
			payload: M.clone(), // proper message
			signature: <Test as Config>::Signature::decode(&mut [0u8; 48].as_ref()).unwrap(),
		};
		assert!(
			TheaHandler::validate_unsigned(TransactionSource::Local, &bad_message_call).is_err()
		);
		// bad nonce
		let mut vs = M_KC.clone();
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::none(),
				vec!(u128::MAX),
				vs.clone(),
				get_valid_signature::<Test>()
			),
			Error::<Test>::MessageNonce
		);
		vs.nonce += 1;
		// Error Deconding keychange
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::none(),
				vec!(u128::MAX),
				vs.clone(),
				get_valid_signature::<Test>()
			),
			Error::<Test>::ErrorDecodingValidatorSet
		);
		// invalid validator set id
		let validators = ValidatorSet { validators: vec![1u64, 2u64, 3u64], set_id: 1 };
		let encoded = validators.encode();
		assert_eq!(validators, ValidatorSet::decode(&mut encoded.as_ref()).unwrap());
		vs.data = encoded.clone();
		assert_eq!(vs.data, encoded);
		assert_noop!(
			TheaHandler::incoming_message(
				RuntimeOrigin::none(),
				vec!(u128::MAX),
				vs.clone(),
				get_valid_signature::<Test>()
			),
			Error::<Test>::InvalidValidatorSetId
		);
		// proper validator set change
		<ValidatorSetId<Test>>::set(0);
		assert_eq!(<ValidatorSetId<Test>>::get(), 0);
		assert_ok!(TheaHandler::incoming_message(
			RuntimeOrigin::none(),
			vec!(u128::MAX),
			vs.clone(),
			get_valid_signature::<Test>()
		));
		// actually inserted
		assert_eq!(<Authorities<Test>>::get(1).len(), 1);
		// event validation
		assert_last_event::<Test>(Event::<Test>::TheaMessageExecuted { message: vs }.into());
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
		// nonce already processed
		assert_noop!(
			TheaHandler::update_incoming_nonce(RuntimeOrigin::root(), u64::MAX),
			Error::<Test>::NonceIsAlreadyProcessed
		);
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
		// nonce already processed
		assert_noop!(
			TheaHandler::update_outgoing_nonce(RuntimeOrigin::root(), u64::MAX),
			Error::<Test>::NonceIsAlreadyProcessed
		);
	})
}
