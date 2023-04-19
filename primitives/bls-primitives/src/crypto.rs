#[cfg(feature = "std")]
use crate::{Error, Pair as BLSPair};
use crate::{KeyStore, Public, Seed, Signature, DST};
#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;

#[cfg(feature = "std")]
use sp_core::crypto::{ExposeSecret, SecretUri};
#[cfg(feature = "std")]
use sp_core::Pair;
use sp_runtime_interface::runtime_interface;

use sp_std::vec::Vec;

pub const BLS_KEYSTORE_PATH: &str = "polkadex/.keystore/";

#[runtime_interface]
pub trait BlsExt {
	#[allow(clippy::result_unit_err)]
	fn add_signature(agg_signature: &Signature, new: &Signature) -> Result<Signature, ()> {
		add_signature(agg_signature, new)
	}

	fn all() -> Vec<Public> {
		// Load all available bls public keys from filesystem
		match get_all_public_keys() {
			Ok(keys) => keys,
			Err(_) => Vec::new(),
		}
	}

	fn generate_pair(phrase: Option<Vec<u8>>) -> Public {
		// generate a pair
		let (pair, _seed, _derive_junctions) = generate_pair_(phrase);
		pair.public()
	}

	fn generate_pair_and_store(phrase: Option<Vec<u8>>) -> Public {
		let (pair, seed, derive_junctions) = generate_pair_(phrase);
		// create keystore
		let keystore: KeyStore = KeyStore::new(seed, derive_junctions);
		// store the private key in filesystem
		let file_path = key_file_path(pair.public().as_ref());
		write_to_file(file_path, keystore.encode().as_ref()).expect("Unable to write seed to file");
		pair.public()
	}

	fn sign(pubkey: &Public, msg: &[u8]) -> Option<Signature> {
		// load the private key from filesystem and sign with it
		sign(pubkey, msg)
	}

	fn verify(pubkey: &Public, msg: &[u8], signature: &Signature) -> bool {
		let pubkey = match PublicKey::uncompress(pubkey.0.as_ref()) {
			Ok(pubkey) => pubkey,
			Err(_) => return false,
		};
		let signature = match crate::BLSSignature::uncompress(signature.0.as_ref()) {
			Ok(sig) => sig,
			Err(_) => return false,
		};
		// verify the signature
		let err = signature.verify(true, msg, DST.as_ref(), &[], &pubkey, true);
		err == BLST_ERROR::BLST_SUCCESS
	}

	fn verify_aggregate(pubkey: &[Public], msg: &[u8], signature: &Signature) -> bool {
		verify_aggregate_(pubkey, msg, signature)
	}
}

#[cfg(feature = "std")]
pub fn add_signature_(sig1: &Signature, sig2: &Signature) -> Result<Signature, ()> {
	let agg_signature = match crate::BLSSignature::from_bytes(sig1.0.as_ref()) {
		Ok(sig) => sig,
		Err(_) => return Err(()),
	};
	let new = match crate::BLSSignature::from_bytes(sig2.0.as_ref()) {
		Ok(sig) => sig,
		Err(_) => return Err(()),
	};
	let mut agg_signature = AggregateSignature::from_signature(&agg_signature);
	if agg_signature.add_signature(&new, true).is_err() {
		return Err(())
	}
	Ok(Signature::from(crate::BLSSignature::from_aggregate(&agg_signature)))
}

#[cfg(feature = "std")]
pub fn verify_aggregate_(pubkey: &[Public], msg: &[u8], signature: &Signature) -> bool {
	let mut pubkeys = vec![];
	for key in pubkey {
		let agg_pubkey = match PublicKey::uncompress(key.0.as_ref()) {
			Ok(pubkey) => pubkey,
			Err(_) => return false,
		};
		pubkeys.push(agg_pubkey);
	}
	let pubkeys_ref = pubkeys.iter().collect::<Vec<&PublicKey>>();

	let agg_signature = match crate::BLSSignature::uncompress(signature.0.as_ref()) {
		Ok(sig) => sig,
		Err(_) => return false,
	};
	// verify the signature
	let err = agg_signature.fast_aggregate_verify(true, msg, DST.as_ref(), &pubkeys_ref);
	err == BLST_ERROR::BLST_SUCCESS
}

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::Write;

use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use sp_core::DeriveJunction;
#[cfg(feature = "std")]
use std::path::PathBuf;
#[cfg(feature = "std")]
use std::str::FromStr;

#[cfg(feature = "std")]
fn generate_pair_(phrase: Option<Vec<u8>>) -> (BLSPair, Seed, Vec<DeriveJunction>) {
	let (pair, seed, derive_junctions) = match phrase {
		None => {
			let (pair, seed) = BLSPair::generate();
			(pair, seed, vec![])
		},
		Some(phrase) => {
			let phrase = String::from_utf8(phrase).expect("Invalid phrase");
			let uri = SecretUri::from_str(phrase.as_ref()).expect("expected a valid phrase");
			let (pair, seed) = BLSPair::from_phrase(uri.phrase.expose_secret(), None)
				.expect("Phrase is not valid; qed");

			let (pair, seed) = pair
				.derive(uri.junctions.iter().cloned(), Some(seed))
				.expect("Expected to derive the pair here.");
			(pair, seed.unwrap(), uri.junctions)
		},
	};
	(pair, seed, derive_junctions)
}

#[cfg(feature = "std")]
#[allow(dead_code)]
pub fn sign(pubkey: &Public, msg: &[u8]) -> Option<Signature> {
	let path = key_file_path(pubkey.as_ref());
	match std::fs::read(&path) {
		Err(err) => {
			log::error!(target:"bls","Error while reading keystore file: {:?}",err);
			None
		},
		Ok(data) => match serde_json::from_slice::<Vec<u8>>(&data) {
			Ok(seed) =>
				return match KeyStore::decode(&mut seed.as_ref()) {
					Ok(keystore) => {
						if let Ok(secret_key) =
							SecretKey::key_gen(keystore.get_seed().as_ref(), &[])
						{
							let mut master_key = secret_key;
							for junction in keystore.get_junctions() {
								let index_bytes = [
									junction.inner()[0],
									junction.inner()[1],
									junction.inner()[2],
									junction.inner()[3],
								];
								master_key =
									master_key.derive_child_eip2333(u32::from_be_bytes(index_bytes))
							}
							return Some(Signature::from(master_key.sign(msg, DST.as_ref(), &[])))
						} else {
							log::error!(target: "bls", "KeyStore has been corrupted, Unable to derive BLS Key");
							None
						}
					},
					Err(err) => {
						log::error!(target:"bls","Error while loading keystore from storage {:?}",err);
						None
					},
				},
			Err(_) => None,
		},
	}
}

#[cfg(feature = "std")]
#[allow(dead_code)]
fn get_all_public_keys() -> Result<Vec<Public>, Error> {
	let mut public_keys = vec![];
	for entry in std::fs::read_dir(BLS_KEYSTORE_PATH)? {
		let entry = entry?;
		let path = entry.path();

		// skip directories and non-unicode file names (hex is unicode)
		if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
			match hex::decode(name) {
				Ok(ref hex) if hex.len() == 96 => {
					let public = hex.to_vec();

					match PublicKey::uncompress(public.as_ref()) {
						Ok(public) => public_keys.push(Public::from(public.to_bytes())),
						Err(_) => continue,
					}
				},
				_ => continue,
			}
		}
	}
	Ok(public_keys)
}

/// Write the given `data` to `file`.
#[cfg(feature = "std")]
#[allow(dead_code)]
fn write_to_file(path: PathBuf, data: &[u8]) -> Result<(), Error> {
	std::fs::create_dir_all(BLS_KEYSTORE_PATH)?;
	let mut file = std::fs::OpenOptions::new().write(true).create(true).open(path)?;
	use std::os::unix::fs::PermissionsExt;
	file.metadata()?.permissions().set_mode(0o600);
	serde_json::to_writer(&file, data)?;
	file.flush()?;
	Ok(())
}

/// Get the file path for the given public key and key type.
///
/// Returns `None` if the keystore only exists in-memory and there isn't any path to provide.
#[cfg(feature = "std")]
#[allow(dead_code)]
fn key_file_path(public: &[u8]) -> PathBuf {
	let mut buf = PathBuf::from(BLS_KEYSTORE_PATH);
	let key = hex::encode(public);
	buf.push(key.as_str());
	buf
}

/// Get the key phrase for a given public key and key type.
#[cfg(feature = "std")]
#[allow(dead_code)]
fn key_phrase_by_type(public: &[u8]) -> Result<Option<String>, Error> {
	let path = key_file_path(public);

	if path.exists() {
		let file = File::open(path)?;

		serde_json::from_reader(&file).map_err(Into::into).map(Some)
	} else {
		Ok(None)
	}
}
