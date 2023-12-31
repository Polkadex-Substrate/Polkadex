use crate::{
	pallet::{
		Authorities, KeygenR1, KeygenR2, LastSigningStage, NextAuthorities, NextTheaPublicKey,
		SigningR1, SigningR2, ValidatorSetId,
	},
	Call, Config, Pallet,
};
use frame_support::traits::Len;
use frame_system::offchain::SubmitTransaction;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::offchain::storage::StorageValueRef;
use std::collections::BTreeMap;
use thea_primitives::{
	types::{AggregatedPayload, OnChainMessage},
	ValidatorSetId as Id,
};

const KEYGEN_R1: [u8; 9] = *b"keygen-r1";
const KEYGEN_R2: [u8; 9] = *b"keygen-r2";
const KEY_PACKAGE: [u8; 17] = *b"keypackage-index-";
const PUBLIC_KEY_PACKAGE: [u8; 24] = *b"public-keypackage-index-";
const SIGNING_R1: [u8; 10] = *b"signing-r1";
const SIGNING_R2: [u8; 10] = *b"signing-r2";

#[derive(Decode, Encode, Copy, Clone, TypeInfo)]
pub enum KeygenStages {
	R1,
	R2,
	R3(Id),
	Key(Id, [u8; 65]),
}

#[derive(Decode, Encode, Clone, TypeInfo)]
pub enum SigningStages {
	None,
	R1(AggregatedPayload),
	R2(AggregatedPayload),
}

impl Default for SigningStages {
	fn default() -> Self {
		Self::None
	}
}

impl<T: Config> Pallet<T> {
	pub fn submit_transaction(
		auth_index: u16,
		signer: T::TheaId,
		message: OnChainMessage,
	) -> Result<(), &'static str> {
		let encoded_payload = message.encode();
		let msg_hash = sp_io::hashing::blake2_256(&encoded_payload);
		let signature = signer.sign(&msg_hash).ok_or("error while signing ecdsa signature")?;

		let call = Call::<T>::handle_thea_2_message {
			auth_index,
			payload: message,
			signature: signature.into(),
		};
		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into()).map_err(|()| {
			log::error!(target: "thea","Unable to submit thea unsigned txn");
			"Unable to submit thea unsigned txn"
		})
	}
	pub fn load_validator_signing_key() -> Result<(u16, T::TheaId), &'static str> {
		let id = <ValidatorSetId<T>>::get();
		let authorities = <Authorities<T>>::get(id).to_vec();

		let local_keys = T::TheaId::all();

		let mut available_keys = authorities
			.iter()
			.enumerate()
			.filter_map(move |(index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| (index as u16, local_keys[location].clone()))
			})
			.collect::<Vec<(u16, T::TheaId)>>();

		available_keys.sort();

		if available_keys.is_empty() {
			return Err("No active keys available")
		}

		available_keys.first().cloned().ok_or("Key not avaialble")
	}

	pub fn load_next_validator_signing_key() -> Result<(u16, T::TheaId), &'static str> {
		let authorities = <NextAuthorities<T>>::get().to_vec();
		let local_keys = T::TheaId::all();

		let mut available_keys = authorities
			.iter()
			.enumerate()
			.filter_map(move |(index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| (index as u16, local_keys[location].clone()))
			})
			.collect::<Vec<(u16, T::TheaId)>>();

		available_keys.sort();

		if available_keys.is_empty() {
			return Err("No active keys available")
		}

		available_keys.first().cloned().ok_or("Key not avaialble")
	}

	pub fn run_thea_frost_logic() -> Result<(), &'static str> {
		log::debug!(target:"thea","Starting thea frost logic...");
		// 1. Check if its time for keygen then do that
		Self::frost_keygen()?;
		// 2. Check pending outgoing messages and sign them
		Self::sign_messages()?;
		Ok(())
	}

	pub fn sign_messages() -> Result<(), &'static str> {
		let (current_auth_index, current_signer) = Self::load_validator_signing_key()?;

		match <LastSigningStage<T>>::get() {
			SigningStages::None => Self::start_new_signing_round()?,
			SigningStages::R1(agg_payload) => Self::complete_signing_round2(agg_payload)?,
			SigningStages::R2(agg_payload) => Self::aggregate_signature_shares(agg_payload)?,
		}

		Ok(())
	}

	pub fn aggregate_signature_shares(
		aggregated_payload: AggregatedPayload,
	) -> Result<(), &'static str> {
		let id = <ValidatorSetId<T>>::get();
		let message = aggregated_payload.root().0;
		let mut key = PUBLIC_KEY_PACKAGE.to_vec();
		key.append(&mut id.encode());
		let storage = StorageValueRef::persistent(&key);
		let public_key_package = match storage.get::<Vec<u8>>() {
			Ok(Some(key_package)) => key_package,
			Ok(None) => return Err("Private key package not found"),
			Err(err) => {
				log::error!(target:"thea","private key package retrieval error: {:?}",err);
				return Err("private key package retrieval error")
			},
		};

		let storage = StorageValueRef::persistent(&SIGNING_R2);
		let encoded_signing_package = match storage.get::<Vec<u8>>() {
			Ok(Some(encoded_signing_package)) => encoded_signing_package,
			Ok(None) => return Err("Private encoded_signing_package not found"),
			Err(err) => {
				log::error!(target:"thea","private encoded_signing_package retrieval error: {:?}",err);
				return Err("private encoded_signing_package retrieval error")
			},
		};

		let params_for_contract = thea_primitives::frost::aggregate(
			encoded_signing_package,
			<SigningR2<T>>::get(),
			public_key_package,
			message,
		)
		.map_err(|()| "Error while aggregating signatures")?;
		// Submit params to on-chain storage
		let (auth_index, signer) = Self::load_validator_signing_key()?;
		Self::submit_transaction(auth_index, signer, OnChainMessage::SR3(params_for_contract))?;
		Ok(())
	}

	pub fn complete_signing_round2(
		aggregated_payload: AggregatedPayload,
	) -> Result<(), &'static str> {
		let id = <ValidatorSetId<T>>::get();
		let message = aggregated_payload.root().0;
		let mut key = KEY_PACKAGE.to_vec();
		key.append(&mut id.encode());

		let storage = StorageValueRef::persistent(&key);
		let key_package = match storage.get::<Vec<u8>>() {
			Ok(Some(key_package)) => key_package,
			Ok(None) => return Err("Private key package not found"),
			Err(err) => {
				log::error!(target:"thea","private key package retrieval error: {:?}",err);
				return Err("private key package retrieval error")
			},
		};

		let storage = StorageValueRef::persistent(&SIGNING_R1);
		let encoded_signing_nonce = match storage.get::<Vec<u8>>() {
			Ok(Some(signing_nonce)) => signing_nonce,
			Ok(None) => return Err("Private signing_nonce not found"),
			Err(err) => {
				log::error!(target:"thea","private signing_nonce retrieval error: {:?}",err);
				return Err("private signing_nonce retrieval error")
			},
		};

		let (signature_share, signing_package) = thea_primitives::frost::sign(
			<SigningR1<T>>::get(),
			encoded_signing_nonce,
			key_package,
			message,
		)
		.map_err(|()| "Error while signing thea message")?;

		let storage = StorageValueRef::persistent(&SIGNING_R2);
		storage.set(&signing_package);

		// Submit on-chain
		let (auth_index, signer) = Self::load_validator_signing_key()?;
		Self::submit_transaction(auth_index, signer, OnChainMessage::SR2(signature_share))?;
		Ok(())
	}

	pub fn start_new_signing_round() -> Result<(), &'static str> {
		let id = <ValidatorSetId<T>>::get();
		let mut key = KEY_PACKAGE.to_vec();
		key.append(&mut id.encode());

		let storage = StorageValueRef::persistent(&key);
		let key_package = match storage.get::<Vec<u8>>() {
			Ok(Some(key_package)) => key_package,
			Ok(None) => return Err("Private key package not found"),
			Err(err) => {
				log::error!(target:"thea","private key package retrieval error: {:?}",err);
				return Err("private key package retrieval error")
			},
		};

		let (nonces, commitments) = thea_primitives::frost::nonce_commit(key_package)
			.map_err(|_| "Error generating nonce and commitments for signing")?;

		let storage = StorageValueRef::persistent(&SIGNING_R1);
		storage.set(&nonces);

		// Submit commitments and aggregate payload on-chain
		let (auth_index, signer) = Self::load_validator_signing_key()?;
		Self::submit_transaction(auth_index, signer, OnChainMessage::SR1(commitments))?;
		Ok(())
	}

	pub fn frost_keygen() -> Result<(), &'static str> {
		let (auth_index, signer) = Self::load_next_validator_signing_key()?;
		let max_signers = <NextAuthorities<T>>::get().len() as u16;
		let min_signers = (2 * max_signers) / 3;
		match <NextTheaPublicKey<T>>::get() {
			None => return Ok(()),
			Some(keygen_stage) => {
				match keygen_stage {
					KeygenStages::R1 => {
						let (r1_secret, r1_broadcast) = thea_primitives::frost::dkg_part1(
							auth_index as u16,
							max_signers,
							min_signers,
						)
						.map_err(|_| "Error while executing dkg_part1")?;
						let storage = StorageValueRef::persistent(&KEYGEN_R1);
						storage.set(&r1_secret);
						// Submit on chain
						let (auth_index, signer) = Self::load_next_validator_signing_key()?;
						Self::submit_transaction(
							auth_index,
							signer,
							OnChainMessage::KR1(r1_broadcast),
						)?;
					},
					KeygenStages::R2 => {
						let storage = StorageValueRef::persistent(&KEYGEN_R1);
						let r1_secret = match storage.get::<Vec<u8>>() {
							Ok(Some(r1_secret)) => r1_secret,
							Ok(None) => return Err("R1 secret not found"),
							Err(err) => {
								log::error!(target:"thea","R1 secret retrieval error: {:?}",err);
								return Err("R1 secret retrieval error")
							},
						};
						let r1_packages = <KeygenR1<T>>::get();
						if r1_packages.len() as u16 != max_signers {
							log::error!(target: "thea","R1 packages submitted: {:?}, required: {:?}",r1_packages.len(),max_signers);
							return Err("All validators didn't submit r1 packages")
						}
						let (r2_secret, encoded_r2) =
							thea_primitives::frost::dkg_part2(&r1_secret, r1_packages)
								.map_err(|_| "Error while executing dkg_part2")?;

						let storage = StorageValueRef::persistent(&KEYGEN_R2);
						storage.set(&r2_secret);

						let r2_broadcast: BTreeMap<[u8; 32], Vec<u8>> =
							Decode::decode(&mut &encoded_r2[..]).map_err(|err| {
								log::error!(target:"thea","Keygen R2 broadcast decode error: {:?}",err);
								"Error while decoding r2 broadcast"
							})?;
						// Submit on-chain
						let (auth_index, signer) = Self::load_next_validator_signing_key()?;
						Self::submit_transaction(
							auth_index,
							signer,
							OnChainMessage::KR2(r2_broadcast),
						)?;
					},
					KeygenStages::R3(id) => {
						let storage = StorageValueRef::persistent(&KEYGEN_R2);
						let r2_secret = match storage.get::<Vec<u8>>() {
							Ok(Some(r2_secret)) => r2_secret,
							Ok(None) => return Err("R2 secret not found"),
							Err(err) => {
								log::error!(target:"thea","R2 secret retrieval error: {:?}",err);
								return Err("R2 secret retrieval error")
							},
						};
						let r1_packages = <KeygenR1<T>>::get();
						let r2_packages = <KeygenR2<T>>::get();
						if r2_packages.len() as u16 != max_signers {
							log::error!(target: "thea","R2 packages submitted: {:?}, required: {:?}",r2_packages.len(),max_signers);
							return Err("All validators didn't submit r2 packages")
						}
						let (key_package, publickey_package, verifying_key) =
							thea_primitives::frost::dkg_part3(&r2_secret, r1_packages, r2_packages)
								.map_err(|_| "Error while executing dkg_part3")?;

						let mut key = KEY_PACKAGE.to_vec();
						key.append(&mut id.encode());

						let storage = StorageValueRef::persistent(&key);
						storage.set(&key_package);

						let mut key = PUBLIC_KEY_PACKAGE.to_vec();
						key.append(&mut id.encode());

						let storage = StorageValueRef::persistent(&key);
						storage.set(&publickey_package);

						// Submit verifying key on chain
						let (auth_index, signer) = Self::load_next_validator_signing_key()?;
						Self::submit_transaction(
							auth_index,
							signer,
							OnChainMessage::VerifyingKey(verifying_key),
						)?;
					},
					KeygenStages::Key(id, key) => return Ok(()),
				}
			},
		}
		Ok(())
	}
}
