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
	fixtures::produce_authorities,
	mock::{new_test_ext, Test, *},
	pallet::*,
};
use frame_support::{
	assert_noop, assert_ok,
	traits::{fungible::Mutate as FungibleMutate, fungibles::Mutate as FungiblesMutate},
};
use frame_system::EventRecord;
use parity_scale_codec::Encode;
use sp_runtime::{
	traits::{AccountIdConversion, BadOrigin},
	SaturatedConversion,
};
use thea_primitives::types::{AssetMetadata, Deposit, Withdraw};

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
