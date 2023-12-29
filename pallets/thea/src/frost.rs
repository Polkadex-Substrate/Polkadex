use crate::{
	pallet::{
		ActiveNetworks, Authorities, KeygenR1, KeygenR2, LastSignedOutgoingNonce, NextAuthorities,
		NextTheaPublicKey, ValidatorSetId,
	},
	Config, Pallet,
};
use frame_support::traits::Len;
use parity_scale_codec::{Decode, Encode};
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::offchain::storage::{StorageRetrievalError, StorageValueRef};
use thea_primitives::ValidatorSetId as Id;

const KEYGEN_R1: [u8; 9] = *b"keygen-r1";
const KEYGEN_R2: [u8; 9] = *b"keygen-r2";
const KEY_PACKAGE: [u8; 17] = *b"keypackage-index-";
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
	R1(u32),
	R2(u32),
	Signed(u32, ([u8; 32], [u8; 32], [u8; 32], [u8; 32])),
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
		match <LastSignedOutgoingNonce<T>>::get() {
			SigningStages::None => {
                // Don't do anything
				return Ok(())
            },
			SigningStages::R1(agg_nonce) => {},
			SigningStages::R2(agg_nonce) => {},
			SigningStages::Signed(agg_nonce, _) => {
				// TODO: Get new aggregated message and start signing
			},
		}
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
						let (key_package, verifying_key) =
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
						// TODO: Submit verifying key on chain
					},
					KeygenStages::Key(id, key) => return Ok(()),
				}
			},
		}
		Ok(())
	}
}
