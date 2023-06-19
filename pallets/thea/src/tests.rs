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
use frame_support::{assert_err, assert_noop, assert_ok, bounded_vec};
use sp_runtime::{AccountId32, DispatchError::BadOrigin, SaturatedConversion, TokenError};

fn any_id() -> <Test as Config>::TheaId {
	<Test as Config>::TheaId::decode(&mut [1u8; 96].as_ref()).unwrap()
}

fn any_signature() -> <Test as Config>::Signature {
	<Test as Config>::Signature::decode(&mut [1u8; 48].as_ref()).unwrap()
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
