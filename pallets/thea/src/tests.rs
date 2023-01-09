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
use frame_support::{assert_noop, assert_ok};
use parity_scale_codec::{Decode, Encode};
use sp_core::{crypto::AccountId32, H160, U256};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{BoundedBTreeSet, BoundedVec, DispatchError::BadOrigin, TokenError};

use crate::{
	mock,
	mock::{new_test_ext, Test, *},
	pallet::*,
};
use blst::min_sig::*;
use thea_primitives::BLSPublicKey;

pub const KEY_TYPE: sp_application_crypto::KeyTypeId = sp_application_crypto::KeyTypeId(*b"ocex");

pub fn set_kth_bit(number: u128, k_value: u8) -> u128 {
	(1 << k_value) | number
}

#[test]
fn test_thea_approve_deposit() {
	let mut ikm = [0 as u8; 32];
	let sk_1 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_1 = sk_1.sk_to_pk();
	let mut ikm = [1 as u8; 32];
	let sk_2 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_2 = sk_2.sk_to_pk();
	let mut ikm = [2 as u8; 32];
	let sk_3 = SecretKey::key_gen(&ikm, &[]).unwrap();
	let pk_3 = sk_3.sk_to_pk();
	let account_id = create_account_id();
	let new_payload = Payload::<u64> {
		network_id: 1,
		who: 2,
		tx_hash: sp_core::H256::default(),
		asset_id: 1,
		amount: 2,
		deposit_nonce: 1,
	};
	let bls_public_key_1 = BLSPublicKey(pk_1.serialize().into());
	let bls_public_key_2 = BLSPublicKey(pk_2.serialize().into());
	let bls_public_key_3 = BLSPublicKey(pk_3.serialize().into());
	let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
	let sig_1 = sk_1.sign(&new_payload.encode(), dst, &[]);
	let sig_2 = sk_2.sign(&new_payload.encode(), dst, &[]);
	let mut agg_sig = AggregateSignature::from_signature(&sig_1);
	agg_sig.add_signature(&sig_2, false).unwrap();
	let sig = agg_sig.to_signature().serialize();
	let mut bit_map_1 = 0_u128;
	bit_map_1 = set_kth_bit(bit_map_1, 0);
	bit_map_1 = set_kth_bit(bit_map_1, 1);
	bit_map_1 = set_kth_bit(bit_map_1, 2);
	let mut bit_map_2 = 0_u128;
	bit_map_2 = set_kth_bit(bit_map_2, 0);
	bit_map_2 = set_kth_bit(bit_map_2, 1);
	new_test_ext().execute_with(|| {
		RelayersBLSKeyVector::<Test>::insert(
			1,
			BoundedVec::try_from(vec![bls_public_key_1, bls_public_key_2, bls_public_key_3])
				.unwrap(),
		);
		// Testing Signature Verification Failure
		assert_noop!(
			Thea::approve_deposit(
				Origin::signed(1),
				bit_map_1,
				sig.into(),
				new_payload.clone().into()
			),
			Error::<Test>::BLSSignatureVerificationFailed
		);
		// Valid Signature
		assert_ok!(Thea::approve_deposit(
			Origin::signed(1),
			bit_map_2,
			sig.into(),
			new_payload.clone().into()
		));
		// Testing Replay Attack
		assert_noop!(
			Thea::approve_deposit(Origin::signed(1), bit_map_2, sig.into(), new_payload.into()),
			Error::<Test>::DepositNonceError
		);
	});
}

fn create_account_id() -> AccountId32 {
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

	return account_id
}
