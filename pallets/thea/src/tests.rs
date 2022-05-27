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

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Unit tests for pallet-thea

use crate::{mock::*, Authorities, MessageLimit};
use codec::Encode;
use frame_support::{
	assert_noop, assert_ok, bounded_vec,
	pallet_prelude::{InvalidTransaction, TransactionSource},
	unsigned::ValidateUnsigned,
};
use sp_application_crypto::RuntimePublic;
use sp_core::sr25519::Signature;
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use sp_runtime::{traits::UniqueSaturatedInto, AccountId32, DispatchError::BadOrigin};

use std::{convert::TryInto, sync::Arc};
use thea_primitives::{
	constants::{MsgLimit, MsgVecLimit, PartialSignatureLimit, PartialSignatureVecLimit},
	keygen::{KeygenRound, Msg, OfflineStageRound, SigningSessionPayload, TheaPayload},
	payload::{Network, SignedTheaPayload, UnsignedTheaPayload},
	AuthorityId, KEY_TYPE,
};

#[test]
fn test_register_deposit_address() {
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

	let test_keytype = sp_application_crypto::KeyTypeId(*b"ethe");
	let test_account_id: AccountId32 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		test_keytype,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair")
	.try_into()
	.expect("Unable to convert to AccountId32");

	new_test_ext().execute_with(|| {
		assert_ok!(Thea::register_deposit_address(Origin::signed(account_id.clone())));

		// Check Storage
		assert_eq!(account_id, Thea::registered_deposit_addresses(account_id.clone()).unwrap());

		// Check Events
		let event: Event = frame_system::pallet::Pallet::<Test>::events()
			.first()
			.expect("Events vector is empty")
			.event
			.clone();

		if let Event::Thea(crate::Event::NewDepositAddressRegistered(val)) = event {
			assert_eq!(val, account_id);
		} else {
			assert!(false, "Wrong event desposited");
		}

		// Lack of Origin
		assert_noop!(Thea::register_deposit_address(Origin::none()), BadOrigin);

		// Root Origin
		assert_noop!(Thea::register_deposit_address(Origin::root()), BadOrigin);

		// This should be failing?
		assert_ok!(Thea::register_deposit_address(Origin::signed(test_account_id.clone())));
	});
}

#[test]
fn test_submit_ecdsa_public_key() {
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

	let public_key = SyncCryptoStore::ecdsa_generate_new(&keystore, KEY_TYPE, None)
		.expect("Could not create ecdsa key pair");
	let uncompressed_public_key: Vec<u8> = vec![1];
	let set_id = 0;
	let call =
		crate::Call::<Test>::submit_ecdsa_public_key { set_id, public_key: public_key.clone() };

	new_test_ext().execute_with(|| {
		assert_ok!(Thea::submit_ecdsa_public_key(Origin::none(), 0, public_key.clone()));

		// Validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &call));

		// Check Storage
		assert_eq!(public_key.clone(), Thea::public_keys(0).unwrap());

		// Check Events
		let event: Event = frame_system::pallet::Pallet::<Test>::events()
			.first()
			.expect("Events vector is empty")
			.event
			.clone();

		if let Event::Thea(crate::Event::ECDSAKeySet(validator_set_id, public_key)) = event {
			assert_eq!(0, validator_set_id);
			assert_eq!(public_key, public_key);
		} else {
			assert!(false, "Wrong event desposited");
		}

		// Signed origin
		assert_noop!(
			Thea::submit_ecdsa_public_key(Origin::signed(account_id), 0, public_key.clone()),
			BadOrigin
		);

		// Root origin
		assert_noop!(
			Thea::submit_ecdsa_public_key(Origin::root(), 0, public_key.clone()),
			BadOrigin
		);
	});
}

/* #[test]
fn test_submit_payload() {
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
	let payload: [u8; 32] = [0; 32];
	let call = crate::Call::<Test>::submit_payload {
		network: Network::ETHEREUM,
		payload: payload.clone(),
		rng: 1,
	};
	new_test_ext().execute_with(|| {
		assert_ok!(Thea::submit_payload(Origin::none(), Network::ETHEREUM, payload.clone(), 1));

		// Validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &call));

		// Check Storage
		assert_eq!(
			payload.clone(),
			UnsignedPayloads::<Test>::get(frame_system::pallet::Pallet::<Test>::block_number())
				.first()
				.expect("UnsignedPayloads vector is empty")
				.payload
		);

		// Network None
		assert_ok!(Thea::submit_payload(Origin::none(), Network::NONE, payload.clone(), 1));

		// signed
		assert_noop!(
			Thea::submit_payload(
				Origin::signed(account_id.clone()),
				Network::ETHEREUM,
				payload.clone(),
				1
			),
			BadOrigin
		);

		// Root signed
		assert_noop!(
			Thea::submit_payload(Origin::root(), Network::ETHEREUM, payload.clone(), 1),
			BadOrigin
		);

		// This extrinsic does not deposit any events
	});
} */

#[test]
fn test_submit_signed_payloads() {
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

	let call = crate::Call::<Test>::submit_signed_payload {
		payload: SignedTheaPayload::default(),
		rng: 1,
	};

	let public_key_store = KeyStore::new();
	let public_key = public_key_store.ecdsa_generate_new(KEY_TYPE, None).unwrap();

	let payload: [u8; 32] = [5; 32];
	let unsigned_payload =
		UnsignedTheaPayload { network: Network::ETHEREUM, payload, submission_blk: 1 };

	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(public_key_store)));

	t.execute_with(|| {
		assert_ok!(Thea::submit_signed_payload(Origin::none(), SignedTheaPayload::default(), 1));

		// Validation
		assert_noop!(
			Thea::validate_unsigned(TransactionSource::Local, &call),
			InvalidTransaction::Call
		);

		// We need ecdsa public key stored to verify signed payload
		assert_ok!(Thea::submit_ecdsa_public_key(Origin::none(), 0, public_key.clone()));

		// Bad Proof Validation
		assert_noop!(
			Thea::validate_unsigned(TransactionSource::Local, &call),
			InvalidTransaction::BadProof
		);

		let signed_payload = SignedTheaPayload {
			payload: unsigned_payload,
			signature: public_key.sign(KEY_TYPE, &payload).expect("Could not create signature"),
		};
		let success_call =
			crate::Call::<Test>::submit_signed_payload { payload: signed_payload, rng: 1 };

		// Successful Validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &success_call));

		// Account signed
		assert_noop!(
			Thea::submit_signed_payload(
				Origin::signed(account_id.clone()),
				SignedTheaPayload::default(),
				1
			),
			BadOrigin
		);

		// Root signed
		assert_noop!(
			Thea::submit_signed_payload(Origin::root(), SignedTheaPayload::default(), 1),
			BadOrigin
		);
	});
}

#[test]
fn test_submit_keygen_message() {
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

	let authority_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let payl: [u8; 64] = [0; 64];
	let sig = Signature::from_raw(payl);
	let payload = TheaPayload::<AuthorityId, KeygenRound, MsgLimit, MsgVecLimit>::default();
	let call = crate::Call::<Test>::submit_keygen_message {
		payload: payload.clone(),
		signature: sig.clone().try_into().expect("Could not convert signature"),
		rng: 1,
	};

	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.execute_with(|| {
		// This will fail, TODO! Need to update with failure message - SignerNotFound
		/* assert_ok!(Thea::submit_keygen_message(
			Origin::none(),
			payload.clone(),
			sig.clone().try_into().expect("Could not convert signature"),
			1
		)); */

		// Update Authorites and Message limit
		let authorities: &[AuthorityId] = &[authority_id.clone().into()];
		let limit: u64 = authorities.len().unique_saturated_into();
		Authorities::<Test>::put(authorities);
		MessageLimit::<Test>::put(limit);

		// Invalid Transaction - BadProof
		assert_noop!(
			Thea::validate_unsigned(TransactionSource::Local, &call),
			InvalidTransaction::BadProof
		);

		let msg = Msg { receiver: None, message: bounded_vec![1, 2, 3], sender: 2 };
		let success_payload: TheaPayload<AuthorityId, KeygenRound, MsgLimit, MsgVecLimit> =
			TheaPayload {
				messages: bounded_vec![msg],
				signer: Some(authority_id.into()),
				set_id: 0,
				auth_idx: 0,
				round: KeygenRound::Unknown,
			};
		let message_hash = sp_io::hashing::keccak_256(&success_payload.encode());
		let signature: thea_primitives::crypto::Signature =
			authority_id.sign(KEY_TYPE, &message_hash).unwrap().into();

		let success_call = crate::Call::<Test>::submit_keygen_message {
			payload: success_payload.clone(),
			signature: signature.clone(),
			rng: 1,
		};
		// Successful Validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &success_call));

		// Succesful Submission
		assert_ok!(Thea::submit_keygen_message(
			Origin::none(),
			success_payload.clone(),
			signature,
			1
		));

		// Check Storage
		assert_eq!(success_payload.clone(), Thea::keygen_messages(payload.auth_idx, payload.round));

		// Check Events
		let event: Event = frame_system::pallet::Pallet::<Test>::events()
			.first()
			.expect("Events vector is empty")
			.event
			.clone();

		if let Event::Thea(crate::Event::KeygenMessages(_thea_id, thea_payload)) = event {
			assert_eq!(success_payload, thea_payload);
		} else {
			assert!(false, "Wrong event desposited");
		}

		// Account signed
		assert_noop!(
			Thea::submit_keygen_message(
				Origin::signed(account_id.clone()),
				payload.clone(),
				sig.clone().try_into().expect("Could not convert signature"),
				1
			),
			BadOrigin
		);

		// Root signed
		assert_noop!(
			Thea::submit_keygen_message(
				Origin::root(),
				payload.clone(),
				sig.clone().try_into().expect("Could not convert signature"),
				1
			),
			BadOrigin
		);
	});
}

#[test]
fn test_clean_keygen_message() {
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

	let authority_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let payl: [u8; 64] = [0; 64];
	let sig = Signature::from_raw(payl);
	let auth_idx = 0;
	let call = crate::Call::<Test>::clean_keygen_messages {
		auth_idx: 0,
		signature: sig.clone().try_into().expect("Could not convert signature"),
		rng: 1,
	};
	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.execute_with(|| {
		assert_ok!(Thea::clean_keygen_messages(
			Origin::none(),
			0,
			sig.clone().try_into().expect("Could not convert signature"),
			1
		));

		// Update Authorites
		let authorities: &[AuthorityId] = &[authority_id.clone().into()];
		Authorities::<Test>::put(authorities);

		// Validation
		assert_noop!(
			Thea::validate_unsigned(TransactionSource::Local, &call),
			InvalidTransaction::BadProof
		);

		let message_hash = sp_io::hashing::keccak_256(&auth_idx.encode());
		let signature: thea_primitives::crypto::Signature =
			authority_id.sign(KEY_TYPE, &message_hash).unwrap().into();
		let success_call =
			crate::Call::<Test>::clean_keygen_messages { auth_idx, signature, rng: 1 };

		// Succesfull validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &success_call));

		// Account Signed
		assert_noop!(
			Thea::clean_keygen_messages(
				Origin::signed(account_id.clone()),
				0,
				sig.clone().try_into().expect("Could not convert signature"),
				1
			),
			BadOrigin
		);

		// Root Signed
		assert_noop!(
			Thea::clean_keygen_messages(
				Origin::root(),
				0,
				sig.clone().try_into().expect("Could not convert signature"),
				1
			),
			BadOrigin
		);
	});
}

#[test]
fn test_submit_offline_message() {
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

	let authority_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");

	let payl: [u8; 64] = [0; 64];
	let sig = Signature::from_raw(payl);
	let payload = TheaPayload::<AuthorityId, OfflineStageRound, MsgLimit, MsgVecLimit>::default();
	let payload_array: [u8; 32] = [0; 32];
	let call = crate::Call::submit_offline_message {
		payload: payload.clone(),
		payload_array: payload_array.clone(),
		signature: sig.clone().try_into().expect("Could not convert siganture"),
		rng: 1,
	};

	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.execute_with(|| {
		// This will fail, TODO! Update with error message SignerNotFound
		/* assert_ok!(Thea::submit_offline_message(
			Origin::none(),
			payload.clone(),
			payload_array.clone(),
			sig.clone().try_into().expect("Could not convert signatures"),
			1
		));*/

		// Update Authorites and Message limit
		let authorities: &[AuthorityId] = &[authority_id.clone().into()];
		let limit: u64 = authorities.len().unique_saturated_into();
		Authorities::<Test>::put(authorities);
		MessageLimit::<Test>::put(limit);

		// Validation
		assert_noop!(
			Thea::validate_unsigned(TransactionSource::Local, &call),
			InvalidTransaction::BadProof
		);

		let msg = Msg { receiver: None, message: bounded_vec![1, 2, 3], sender: 2 };
		let success_payload: TheaPayload<AuthorityId, OfflineStageRound, MsgLimit, MsgVecLimit> =
			TheaPayload {
				messages: bounded_vec![msg],
				signer: Some(authority_id.into()),
				set_id: 0,
				auth_idx: 0,
				round: OfflineStageRound::Unknown,
			};
		let message_hash = sp_io::hashing::keccak_256(&success_payload.encode());
		let signature: thea_primitives::crypto::Signature =
			authority_id.sign(KEY_TYPE, &message_hash).unwrap().into();

		let success_call = crate::Call::<Test>::submit_offline_message {
			payload: success_payload.clone(),
			payload_array: payload_array.clone(),
			signature: signature.clone(),
			rng: 1,
		};
		// Successful Validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &success_call));

		// Succesful Submission
		assert_ok!(Thea::submit_offline_message(
			Origin::none(),
			success_payload.clone(),
			payload_array.clone(),
			signature.clone(),
			1
		));

		// Check Storage
		// assert_eq!(success_payload.clone(), Thea::offline_messages(payload.auth_idx,
		// payload.round));

		// Check Events
		let event: Event = frame_system::pallet::Pallet::<Test>::events()
			.first()
			.expect("Events vector is empty")
			.event
			.clone();

		if let Event::Thea(crate::Event::OfflineMessages(_thea_id, thea_payload)) = event {
			assert_eq!(success_payload, thea_payload);
		} else {
			assert!(false, "Wrong event desposited");
		}

		// Account signed
		assert_noop!(
			Thea::submit_offline_message(
				Origin::signed(account_id),
				payload.clone(),
				payload_array.clone(),
				sig.clone().try_into().expect("Could not convert signatures"),
				1
			),
			BadOrigin
		);

		// Root signed
		assert_noop!(
			Thea::submit_offline_message(
				Origin::root(),
				payload.clone(),
				payload_array.clone(),
				sig.clone().try_into().expect("Could not convert signatures"),
				1
			),
			BadOrigin
		);
	});
}

#[test]
fn test_submit_signing_message() {
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

	let authority_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let payl: [u8; 64] = [0; 64];
	let sig = Signature::from_raw(payl);
	let payload = SigningSessionPayload::<
		AuthorityId,
		PartialSignatureLimit,
		PartialSignatureVecLimit,
	>::default();

	let mut t = new_test_ext();
	t.register_extension(KeystoreExt(Arc::new(keystore)));
	t.execute_with(|| {
		let call = crate::Call::submit_signing_message {
			at: frame_system::pallet::Pallet::<Test>::block_number(),
			payload: payload.clone(),
			signature: sig.clone().try_into().expect("Could not convert signature"),
			rng: 1,
		};

		// This fails
		/* assert_ok!(Thea::submit_signing_message(
			Origin::none(),
			frame_system::pallet::Pallet::<Test>::block_number(),
			payload.clone(),
			sig.clone().try_into().expect("Could not convert signature"),
			1
		)); */

		// Update Authorites
		let authorities: &[AuthorityId] = &[authority_id.clone().into()];
		Authorities::<Test>::put(authorities);

		// Validation
		assert_noop!(
			Thea::validate_unsigned(TransactionSource::Local, &call),
			InvalidTransaction::BadProof
		);

		let success_payload: SigningSessionPayload<
			AuthorityId,
			PartialSignatureLimit,
			PartialSignatureVecLimit,
		> = SigningSessionPayload {
			partial_signatures: bounded_vec![],
			signer: Some(authority_id.into()),
			set_id: 0,
			auth_idx: 0,
		};
		let message_hash = sp_io::hashing::keccak_256(&success_payload.encode());
		let signature: thea_primitives::crypto::Signature =
			authority_id.sign(KEY_TYPE, &message_hash).unwrap().into();
		let success_call = crate::Call::submit_signing_message {
			at: frame_system::pallet::Pallet::<Test>::block_number(),
			payload: success_payload.clone(),
			signature: signature.clone(),
			rng: 1,
		};
		// Succesful validation
		assert_ok!(Thea::validate_unsigned(TransactionSource::Local, &success_call));

		// Succesful submission
		assert_ok!(Thea::submit_signing_message(
			Origin::none(),
			frame_system::pallet::Pallet::<Test>::block_number(),
			success_payload.clone(),
			signature,
			1
		));

		// Check Storage
		assert_eq!(
			success_payload.clone(),
			Thea::signing_messages(
				frame_system::pallet::Pallet::<Test>::block_number(),
				payload.auth_idx
			)
		);

		// Check Events
		let event: Event = frame_system::pallet::Pallet::<Test>::events()
			.first()
			.expect("Events vector is empty")
			.event
			.clone();

		if let Event::Thea(crate::Event::SigningMessages(_thea_id, thea_payload)) = event {
			assert_eq!(success_payload, thea_payload);
		} else {
			assert!(false, "Wrong event desposited");
		}

		// Account signed
		assert_noop!(
			Thea::submit_signing_message(
				Origin::signed(account_id),
				frame_system::pallet::Pallet::<Test>::block_number(),
				payload.clone(),
				sig.clone().try_into().expect("Could not convert signature"),
				1
			),
			BadOrigin
		);

		// Root signed
		assert_noop!(
			Thea::submit_signing_message(
				Origin::root(),
				frame_system::pallet::Pallet::<Test>::block_number(),
				payload.clone(),
				sig.clone().try_into().expect("Could not convert signature"),
				1
			),
			BadOrigin
		);
	});
}

#[test]
fn test_keygen_message_store_clear() {
	let payl: [u8; 64] = [0; 64];
	let sig = Signature::from_raw(payl);
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let authority_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let msg = Msg { receiver: None, message: bounded_vec![1, 2, 3], sender: 2 };
	let payload: TheaPayload<AuthorityId, KeygenRound, MsgLimit, MsgVecLimit> = TheaPayload {
		messages: bounded_vec![msg],
		signer: Some(authority_id.into()),
		set_id: 0,
		auth_idx: 0,
		round: KeygenRound::Unknown,
	};

	new_test_ext().execute_with(|| {
		// Store keygen message
		assert_ok!(Thea::submit_keygen_message(
			Origin::none(),
			payload.clone(),
			sig.clone().into(),
			1
		));

		// Check Storage
		assert_eq!(payload, Thea::keygen_messages(0, KeygenRound::Unknown));

		// Clean keygen messages
		assert_ok!(Thea::clean_keygen_messages(
			Origin::none(),
			0,
			sig.clone().try_into().expect("Could not convert signature"),
			1
		));

		// Check if the message has been cleaned
		assert_eq!(
			TheaPayload::<AuthorityId, KeygenRound, MsgLimit, MsgVecLimit>::default(),
			Thea::keygen_messages(0, KeygenRound::Unknown)
		);
	})
}
/* #[test]
fn test_submit_offense() {
	const PHRASE: &str =
		"news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let keystore = KeyStore::new();
	let authority_id = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let authority_id_1 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter2", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let authority_id_3 = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		KEY_TYPE,
		Some(&format!("{}/hunter3", PHRASE)),
	)
	.expect("Unable to create sr25519 key pair");
	let mut report: OffenseReport<AccountId32, KeygenRound> = OffenseReport {
		offender: authority_id_1.into(),
		offense_blk: 0,
		round: KeygenRound::Round0,
		reporters: BTreeSet::new(),
	};
	new_test_ext().execute_with(|| {
		// Testing if it's all okay
		assert_ok!(Thea::register_offense(
			Origin::signed(authority_id.into()),
			report.clone().into()
		));

		// Check storage
		report.reporters.insert(authority_id.into());
		assert_eq!(Thea::offence_report((&report.offender, &report.round)).unwrap(), report);
		// This will fail
		assert_ok!(Thea::register_offense(
			Origin::signed(authority_id_3.into()),
			report.clone().into()
		));
		report.reporters.insert(authority_id_3.into());
		assert_eq!(Thea::offence_report((&report.offender, &report.round)).unwrap(), report);
	});
} */
