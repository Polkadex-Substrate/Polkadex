use crate::{
	pallet::{
		ActiveNetworks, Authorities, KeygenR1, KeygenR2, LastSignedOutgoingNonce, LastSigningStage,
		NextAuthorities, NextTheaPublicKey, OutgoingMessages, SigningR1, SigningR2, ValidatorSetId,
	},
	Config, Pallet,
};
use frame_support::traits::Len;
use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::offchain::storage::{StorageRetrievalError, StorageValueRef};
use std::collections::{BTreeMap, BTreeSet};
use thea_primitives::{types::AggregatedPayload, Message, ValidatorSetId as Id};

const KEYGEN_R1: [u8; 9] = *b"keygen-r1";
const KEYGEN_R2: [u8; 9] = *b"keygen-r2";
const KEY_PACKAGE: [u8; 17] = *b"keypackage-index-";
const PUBLIC_KEY_PACKAGE: [u8; 24] = *b"public-keypackage-index-";
const SIGNING_R1: [u8; 10] = *b"signing-r1";
const SIGNING_R2: [u8; 10] = *b"signing-r2";

#[derive(Decode, Encode, Copy, Clone)]
pub enum KeygenStages {
	R1(Id),
	R2(Id),
	R3(Id),
	Key(Id, [u8; 64]),
}

#[derive(Decode, Encode, Copy, Clone)]
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
	pub fn load_validator_signing_key() -> Result<(u32, T::TheaId), &'static str> {
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
					.map(|location| (index as u32, local_keys[location].clone()))
			})
			.collect::<Vec<(u32, T::TheaId)>>();

		available_keys.sort();

		if available_keys.is_empty() {
			return Err("No active keys available")
		}

		*available_keys.first().ok_or("Key not avaialble")
	}

	pub fn load_next_validator_signing_key() -> Result<(u32, T::TheaId), &'static str> {
		let authorities = <NextAuthorities<T>>::get().to_vec();
		let local_keys = T::TheaId::all();

		let mut available_keys = authorities
			.iter()
			.enumerate()
			.filter_map(move |(index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| (index as u32, local_keys[location].clone()))
			})
			.collect::<Vec<(u32, T::TheaId)>>();

		available_keys.sort();

		if available_keys.is_empty() {
			return Err("No active keys available")
		}

		*available_keys.first().ok_or("Key not avaialble")
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
		let encoded_signature_shares_map = <SigningR2<T>>::get().encode();
		let params_for_contract = thea_primitives::frost::thea_frost_ext::aggregate(
			encoded_signing_package,
			encoded_signature_shares_map,
			public_key_package,
			message,
		)
		.map_err(|()| "Error while aggregating signatures")?;
		// TODO: Submit params to on-chain storage
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

		let encoded_commitments_map = <SigningR1<T>>::get().encode();

		let (signature_share, signing_package) = thea_primitives::frost::thea_frost_ext::sign(
			encoded_commitments_map,
			encoded_signing_nonce,
			key_package,
			message,
		)
		.map_err(|()| "Error while signing thea message")?;

		let storage = StorageValueRef::persistent(&SIGNING_R2);
		storage.set(&signing_package);

		// TODO: Submit signature share on-chain
		Ok(())
	}

	pub fn start_new_signing_round() -> Result<(), &'static str> {
		let active_networks = <ActiveNetworks<T>>::get();
		let id = <ValidatorSetId<T>>::get();
		let mut pending_messages = BTreeSet::new();
		for network in active_networks {
			let next_nonce = <LastSignedOutgoingNonce<T>>::get(network);
			match <OutgoingMessages<T>>::get(network, next_nonce) {
				None => continue,
				Some(msg) => {
					pending_messages.insert(msg.into());
				},
			}
		}
		let aggregated_payload =
			AggregatedPayload { validator_set_id: id, messages: pending_messages };

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

		let (nonces, commitments) =
			thea_primitives::frost::thea_frost_ext::nonce_commit(key_package)
				.map_err(|_| "Error generating nonce and commitments for signing")?;

		let storage = StorageValueRef::persistent(&SIGNING_R1);
		storage.set(&nonces);

		// TODO: Submit commitments and aggregate payload on-chain
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
					KeygenStages::R1(id) => {
						let (r1_secret, r1_broadcast) =
							thea_primitives::frost::thea_frost_ext::dkg_part1(
								auth_index as u16,
								max_signers,
								min_signers,
							)
							.map_err(|_| "Error while executing dkg_part1")?;
						let storage = StorageValueRef::persistent(&KEYGEN_R1);
						storage.set(&r1_secret);
						// TODO: Submit on chain
					},
					KeygenStages::R2(id) => {
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
						if r1_packages.len() != max_signers {
							log::error!(target: "thea","R1 packages submitted: {:?}, required: {:?}",r1_packages.len(),max_signers);
							return Err("All validators didn't submit r1 packages")
						}
						let (r2_secret, r2_broadcast) =
							thea_primitives::frost::thea_frost_ext::dkg_part2(
								&r1_secret,
								r1_packages.encode(),
							)
							.map_err(|_| "Error while executing dkg_part2")?;

						let storage = StorageValueRef::persistent(&KEYGEN_R2);
						storage.set(&r2_secret);
						// TODO: Submit on-chain
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
						if r2_packages.len() != max_signers {
							log::error!(target: "thea","R2 packages submitted: {:?}, required: {:?}",r2_packages.len(),max_signers);
							return Err("All validators didn't submit r2 packages")
						}
						let (key_package, publickey_package, verifying_key) =
							thea_primitives::frost::thea_frost_ext::dkg_part3(
								&r2_secret,
								r1_packages.encode(),
								r2_packages.encode(),
							)
							.map_err(|_| "Error while executing dkg_part3")?;

						let mut key = KEY_PACKAGE.to_vec();
						key.append(&mut id.encode());

						let storage = StorageValueRef::persistent(&key);
						storage.set(&key_package);

						let mut key = PUBLIC_KEY_PACKAGE.to_vec();
						key.append(&mut id.encode());

						let storage = StorageValueRef::persistent(&key);
						storage.set(&publickey_package);

						// TODO: Submit verifying key on chain
					},
					KeygenStages::Key(id, key) => return Ok(()),
				}
			},
		}
		Ok(())
	}
}
