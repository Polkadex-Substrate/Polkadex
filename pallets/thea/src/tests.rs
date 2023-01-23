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
use asset_handler::pallet::TheaAssets;
use frame_support::{assert_noop, assert_ok, ensure};
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
// use frame_support::metadata::StorageEntryModifier::Default as ;
use thea_primitives::{AssetIdConverter, BLSPublicKey, TokenType};
use sp_std::default::Default;
use thea_primitives::parachain_primitives::{ParachainAsset, ParachainDeposit};
use xcm::{
	latest::{Fungibility, MultiAsset, MultiLocation, AssetId, Junction, NetworkId},
	prelude::{Xcm, X1},
};

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
	let multi_asset = MultiAsset {
		id: AssetId::Concrete(MultiLocation::default()),
		fun: Fungibility::Fungible(10_u128)
	};
	let multi_location = MultiLocation {
		parents: 0,
		interior: X1(Junction::AccountId32 { network: NetworkId::Any, id: create_account_id().encode().try_into().unwrap() }),
	};
	let new_payload = ParachainDeposit{
		recipient: multi_location,
		asset_and_amount: multi_asset,
		deposit_nonce: 1,
		transaction_hash: sp_core::H256::zero(),
		network_id: 1
	};
	let asset_id = new_payload.get_asset_id();
    // Register asset
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
		asset_handler::pallet::TheaAssets::<Test>::insert(asset_id.unwrap(), (1, 1, BoundedVec::default()));
		RelayersBLSKeyVector::<Test>::insert(
			1,
			BoundedVec::try_from(vec![bls_public_key_1, bls_public_key_2, bls_public_key_3])
				.unwrap(),
		);
		TheaAssets::<Test>::insert(1, (0, 0, BoundedVec::try_from(vec![]).unwrap()));
		// Testing Signature Verification Failure
		// assert_noop!(
		// 	Thea::approve_deposit(
		// 		Origin::signed(1),
		// 		bit_map_1,
		// 		sig.into(),
		// 		new_payload.clone().into()
		// 	),
		// 	Error::<Test>::BLSSignatureVerificationFailed
		// );
		// Valid Signature
		assert_ok!(Thea::approve_deposit(
			Origin::signed(1),
			bit_map_2,
			sig.into(),
			TokenType::Fungible(1_u8),
			new_payload.encode()
		));
		// Testing Replay Attack
		// assert_noop!(
		// 	Thea::approve_deposit(Origin::signed(1), bit_map_2, sig.into(), new_payload.into()),
		// 	Error::<Test>::DepositNonceError
		// );
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