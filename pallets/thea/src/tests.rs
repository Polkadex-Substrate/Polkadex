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
