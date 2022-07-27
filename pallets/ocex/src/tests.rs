// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

//! Tests for pallet-example-basic.

use crate::*;
use frame_support::{
	parameter_types,
	traits::{ConstU128, ConstU64},
	PalletId,
	assert_noop, assert_ok,
};
use frame_support::traits::OnTimestampSet;
use polkadex_primitives::{Moment, Signature};
use sp_std::cell::RefCell;
use frame_system::EnsureRoot;
use sp_core::H256;
// The testing primitives are very useful for avoiding having to work with signatures
// or public keys. `u64` is used as the `AccountId` and no `Signature`s are required.
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
	AccountId32
};
use crate::mock::*;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

#[test]
fn test_register_main_account(){
	let account_id = create_account_id();

	new_test_ext().execute_with(|| {
		assert_ok!(OCEX::register_main_account(Origin::signed(account_id.clone().into()), account_id.clone().into()));
		assert_eq!(Accounts::<Test>::contains_key::<AccountId32>(account_id.into()), true);
	});
}



fn create_account_id() -> AccountId32{
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	return account_id;
}