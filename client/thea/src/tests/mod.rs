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

//! Thea client testing 

use crate::utils::{convert_all_keygen_messages, convert_back_keygen_messages, convert_signature};
use curv::{
	arithmetic::{BigInt, Converter},
	elliptic::curves::ECPoint,
};
use frame_support::traits::Len;

use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::{
	party_i::{verify, SignatureRecid},
	state_machine::{
		keygen::{Keygen, ProtocolMessage},
		sign::{OfflineStage, SignManual},
	},
};
use round_based::StateMachine;
use sp_core::blake2_256;
use sp_runtime::app_crypto::RuntimePublic;
use thea_primitives::keygen::{KeygenRound, TheaPayload};

// test of thea protocol on runtime level
pub(crate) mod protocol_tests;

#[test]
pub fn test_encode_decode() {
	let mut local_party = Keygen::new(1, 2, 3).unwrap();
	local_party.proceed().unwrap();

	let messages = local_party.message_queue();

	let encoded_messages = convert_all_keygen_messages(messages.clone()).unwrap();
	assert_eq!(encoded_messages.len(), messages.len());
	let payload = TheaPayload {
		messages: encoded_messages,
		set_id: 0,
		auth_idx: 0,
		round: KeygenRound::Unknown,
		..Default::default()
	};
	let decoded_messages =
		convert_back_keygen_messages::<KeygenRound, ProtocolMessage>(payload).unwrap();

	assert_eq!(decoded_messages.len(), messages.len());
	for i in 0..messages.len() {
		assert_eq!(messages[i].sender, decoded_messages[i].sender);
		assert_eq!(messages[i].receiver, decoded_messages[i].receiver);
		// assert_eq!(messages[i].body,decoded_messages[i].body);
	}
}

#[test]
pub fn test_public_key_conversion() {
	let mut alice = Keygen::new(1, 1, 2).unwrap();
	let mut bob = Keygen::new(2, 1, 2).unwrap();

	while !alice.is_finished() && !bob.is_finished() {
		if alice.wants_to_proceed() {
			alice.proceed().unwrap();
		}
		if bob.wants_to_proceed() {
			bob.proceed().unwrap();
		}

		let alice_messages = alice.message_queue().clone();
		alice.message_queue().clear();
		let bob_messages = bob.message_queue().clone();
		bob.message_queue().clear();

		for msg in bob_messages {
			alice.handle_incoming(msg.clone()).unwrap();
		}

		for msg in alice_messages {
			bob.handle_incoming(msg.clone()).unwrap();
		}

		println!("Status => Alice: {:?}, Bob: {:?}", alice.current_round(), bob.current_round());
	}
	let alice_pubk = alice.pick_output().unwrap().unwrap();
	let bob_pubk = bob.pick_output().unwrap().unwrap();

	// We check if the raw uncompressed public keys are equal
	assert_eq!(alice_pubk.public_key(), bob_pubk.public_key());

	// sp_core::ecdsa::Public::from_full creates a compressed ecdsa public key
	let converted_key = sp_core::ecdsa::Public::from_full(
		&alice_pubk.public_key().into_raw().serialize_compressed(),
	)
	.unwrap();
	// alice_pubk.public_key().bytes_compressed_to_big_int() returns a compressed public key
	// alice_pubk.public_key().pk_to_key_slice() returns a uncompressed public key
	assert_eq!(
		alice_pubk.public_key().into_raw().serialize_compressed().to_vec(),
		RuntimePublic::to_raw_vec(&converted_key)
	);
}

#[test]
pub fn test_signature_conversion() {
	// Keygen Stage
	let mut alice = Keygen::new(1, 1, 2).unwrap();
	let mut bob = Keygen::new(2, 1, 2).unwrap();

	while !alice.is_finished() && !bob.is_finished() {
		if alice.wants_to_proceed() {
			alice.proceed().unwrap();
		}
		if bob.wants_to_proceed() {
			bob.proceed().unwrap();
		}

		let alice_messages = alice.message_queue().clone();
		alice.message_queue().clear();
		let bob_messages = bob.message_queue().clone();
		bob.message_queue().clear();

		for msg in bob_messages {
			alice.handle_incoming(msg.clone()).unwrap();
		}

		for msg in alice_messages {
			bob.handle_incoming(msg.clone()).unwrap();
		}

		println!("Status => Alice: {:?}, Bob: {:?}", alice.current_round(), bob.current_round());
	}
	let alice_pubk = alice.pick_output().unwrap().unwrap();
	let bob_pubk = bob.pick_output().unwrap().unwrap();

	// We check if the raw uncompressed public keys are equal
	assert_eq!(
		alice_pubk.public_key().into_raw().serialize_compressed(),
		bob_pubk.public_key().into_raw().serialize_compressed()
	);

	// sp_core::ecdsa::Public::from_full creates a compressed ecdsa public key
	let converted_key = sp_core::ecdsa::Public::from_full(
		&alice_pubk.public_key().into_raw().serialize_compressed(),
	)
	.unwrap();

	// Offline Stage
	let mut alice = OfflineStage::new(1, vec![1, 2], alice_pubk).unwrap();
	let mut bob = OfflineStage::new(2, vec![1, 2], bob_pubk).unwrap();
	while !alice.is_finished() && !bob.is_finished() {
		if alice.wants_to_proceed() {
			alice.proceed().unwrap();
		}
		if bob.wants_to_proceed() {
			bob.proceed().unwrap();
		}

		let alice_messages = alice.message_queue().clone();
		alice.message_queue().clear();
		let bob_messages = bob.message_queue().clone();
		bob.message_queue().clear();

		for msg in bob_messages {
			alice.handle_incoming(msg.clone()).unwrap();
		}

		for msg in alice_messages {
			bob.handle_incoming(msg.clone()).unwrap();
		}

		println!("Status => Alice: {:?}, Bob: {:?}", alice.current_round(), bob.current_round());
	}
	let alice_offline_completed = alice.pick_output().unwrap().unwrap();
	let bob_offline_completed = bob.pick_output().unwrap().unwrap();

	let data: Vec<u8> = vec![
		12, 13, 246, 187, 233, 143, 138, 109, 82, 88, 208, 207, 179, 101, 234, 17, 248, 96, 70,
		158, 195, 155, 200, 25, 83, 70, 7, 177, 132, 223, 246, 85,
	];
	let data_to_sign: [u8; 32] = blake2_256(&data);

	let (alice_sign, alice_msg) =
		SignManual::new(BigInt::from_bytes(&data_to_sign), alice_offline_completed.clone())
			.unwrap();
	let (bob_sign, bob_msg) =
		SignManual::new(BigInt::from_bytes(&data_to_sign), bob_offline_completed.clone()).unwrap();

	let alice_signature: SignatureRecid = alice_sign.complete(&vec![bob_msg]).unwrap();
	let bob_signature = bob_sign.complete(&vec![alice_msg]).unwrap();

	assert_eq!(alice_signature.r, bob_signature.r);
	assert_eq!(alice_signature.s, bob_signature.s);
	assert_eq!(alice_signature.recid, bob_signature.recid);

	assert!(verify(
		&alice_signature,
		alice_offline_completed.public_key(),
		&BigInt::from_bytes(&data_to_sign)
	)
	.is_ok());
	assert!(verify(
		&bob_signature,
		bob_offline_completed.public_key(),
		&BigInt::from_bytes(&data_to_sign)
	)
	.is_ok());

	let converted_alice_signature = convert_signature(&alice_signature).unwrap();
	println!("{:?}", converted_alice_signature);

	assert!(
		thea_primitives::runtime::crypto::verify_ecdsa_prehashed(
			&converted_alice_signature,
			&converted_key,
			&data_to_sign
		),
		"Converted Signature failed"
	);
}
