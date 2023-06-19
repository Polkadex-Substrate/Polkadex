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

//! Tests for pallet-ocex.

use crate::{mock::*, *};
use bls_primitives::{Pair, Public, Signature};
use frame_support::{assert_err, assert_noop, assert_ok, bounded_vec};
use sp_core::Pair as CorePair;
use sp_runtime::{AccountId32, DispatchError::BadOrigin, SaturatedConversion, TokenError};

const WELL_KNOWN: &str = "bottom drive obey lake curtain smoke basket hold race lonely fit walk";

fn any_id() -> <Test as Config>::TheaId {
	<Test as Config>::TheaId::decode(&mut [1u8; 96].as_ref()).unwrap()
}

fn any_signature() -> <Test as Config>::Signature {
	<Test as Config>::Signature::decode(&mut [1u8; 48].as_ref()).unwrap()
}

fn set_200_validators() -> [Pair; 200] {
	let mut validators = Vec::with_capacity(200);
	for i in 0..200 {
		validators[i] =
			Pair::generate_with_phrase(Some(format!("{}//{}", WELL_KNOWN, i).as_str())).0;
	}
	let mut bv: BoundedVec<<Test as Config>::TheaId, <Test as Config>::MaxAuthorities> =
		BoundedVec::with_max_capacity();
	validators
		.clone()
		.into_iter()
		.for_each(|v| bv.try_push(v.public().into()).unwrap());
	<Authorities<Test>>::insert(1, 0, bv);
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
		validator_set_len: 200,
	}
}

#[test]
fn test_update_network_pref_bad_origin() {
	new_test_ext().execute_with(|| {
		assert_err!(
			Thea::update_network_pref(RuntimeOrigin::root(), any_id(), 0, any_signature()),
			BadOrigin
		);
	})
}

#[test]
fn test_update_network_pref_success() {
	new_test_ext().execute_with(|| {
		assert_ok!(Thea::update_network_pref(RuntimeOrigin::none(), any_id(), 0, any_signature()));
	})
}
