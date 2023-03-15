#[cfg(feature = "std")]
use crate::{Error, Pair as BLSPair};
use crate::{Public, Seed, Signature, BLS_DEV_PHRASE, DEV_PHRASE, DST};
#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;
#[cfg(feature = "std")]
use sp_application_crypto::serde::Serialize;
use sp_core::crypto::KeyTypeId;
#[cfg(feature = "std")]
use sp_core::crypto::{ExposeSecret, SecretUri};
#[cfg(feature = "std")]
use sp_core::Pair;
use sp_runtime_interface::runtime_interface;

use sp_std::vec::Vec;

pub const BLS_KEYSTORE_PATH: &str = "/polkadex/.keystore/";

#[runtime_interface]
pub trait BlsExt {
	fn add_signature(agg_signature: &Signature, new: &Signature) -> Result<Signature, ()> {
		let agg_signature = match crate::BLSSignature::from_bytes(agg_signature.0.as_ref()) {
			Ok(sig) => sig,
			Err(_) => return Err(()),
		};
		let new = match crate::BLSSignature::from_bytes(new.0.as_ref()) {
			Ok(sig) => sig,
			Err(_) => return Err(()),
		};
		let mut agg_signature = AggregateSignature::from_signature(&agg_signature);
		if let Err(_) = agg_signature.add_signature(&new, true) {
			return Err(())
		}
		Ok(Signature::from(crate::BLSSignature::from_aggregate(&agg_signature)))
	}

	fn all() -> Vec<Public> {
		// Load all available bls public keys from filesystem
		match get_all_public_keys() {
			Ok(keys) => keys,
			Err(_) => Vec::new(),
		}
	}

	fn generate_pair(phrase: Option<Vec<u8>>) -> Public {
		// generate the private key  and store it in filesystem
		let (pair, _seed) = match phrase {
			None => BLSPair::generate(),
			Some(phrase) => {
				let phrase = String::from_utf8(phrase).expect("Invalid phrase");
				let mut uri =
					SecretUri::from_str(phrase.as_ref()).expect("expected a valid phrase");
				if uri.phrase.expose_secret() == DEV_PHRASE {
					// We want atleast 32 bytes for bls key generation
					uri.phrase = BLS_DEV_PHRASE.parse().unwrap();
				}
				let (pair, seed) = BLSPair::from_phrase(uri.phrase.expose_secret(), None)
					.expect("Phrase is not valid; qed");

				let (pair, seed) = pair
					.derive(uri.junctions.iter().cloned(), Some(seed))
					.expect("Expected to derive the pair here.");
				(pair, seed.unwrap())
			},
		};
		pair.public
	}

	fn sign(pubkey: &Public, msg: &[u8]) -> Option<Signature> {
		// load the private key from filesystem and sign with it
		sign(pubkey, msg)
	}

	fn verify(pubkey: &Public, msg: &[u8], signature: &Signature) -> bool {
		let pubkey = match PublicKey::from_bytes(pubkey.0.as_ref()) {
			Ok(pubkey) => pubkey,
			Err(_) => return false,
		};

		let signature = match crate::BLSSignature::from_bytes(signature.0.as_ref()) {
			Ok(sig) => sig,
			Err(_) => return false,
		};
		// verify the signature
		let err = signature.verify(true, msg, DST.as_ref(), &[], &pubkey, true);
		return if err == BLST_ERROR::BLST_SUCCESS { true } else { false }
	}

	fn verify_aggregate(pubkey: &Vec<Public>, msg: &[u8], signature: &Signature) -> bool {
		let mut pubkeys = vec![];
		for key in pubkey {
			let agg_pubkey = match PublicKey::from_bytes(key.0.as_ref()) {
				Ok(pubkey) => pubkey,
				Err(_) => return false,
			};
			pubkeys.push(agg_pubkey);
		}
		let pubkeys_ref = pubkeys.iter().collect::<Vec<&PublicKey>>();

		let agg_signature = match crate::BLSSignature::from_bytes(signature.0.as_ref()) {
			Ok(sig) => sig,
			Err(_) => return false,
		};
		// verify the signature
		let err = agg_signature.fast_aggregate_verify(true, msg, DST.as_ref(), &pubkeys_ref);
		return if err == BLST_ERROR::BLST_SUCCESS { true } else { false }
	}
}

#[cfg(feature = "std")]
use bip39::{Language, Mnemonic};
#[cfg(feature = "std")]
use sp_core::DeriveJunction;
#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
use std::os::unix::fs;
#[cfg(feature = "std")]
use std::path::PathBuf;
#[cfg(feature = "std")]
use std::str::FromStr;

#[cfg(feature = "std")]
fn sign(pubkey: &Public, msg: &[u8]) -> Option<Signature> {
	let path = key_file_path(pubkey.as_ref());
	match std::fs::read(&path) {
		Err(_) => return None,
		Ok(seed) => match SecretKey::from_bytes(&seed) {
			Ok(secret_key) => Some(Signature::from(secret_key.sign(msg, DST.as_ref(), &[]))),
			Err(_) => return None,
		},
	}
}

#[cfg(feature = "std")]
fn get_all_public_keys() -> Result<Vec<Public>, Error> {
	let mut public_keys = vec![];
	for entry in std::fs::read_dir(&BLS_KEYSTORE_PATH)? {
		let entry = entry?;
		let path = entry.path();

		// skip directories and non-unicode file names (hex is unicode)
		if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
			match hex::decode(name) {
				Ok(ref hex) if hex.len() > 4 => {
					let public = hex.to_vec();
					match Public::try_from(public.as_ref()) {
						Ok(public) => public_keys.push(public),
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
fn write_to_file(file: PathBuf, data: &[u8]) -> Result<(), Error> {
	let mut file = File::create(file)?;
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
fn key_file_path(public: &[u8]) -> PathBuf {
	let mut buf = PathBuf::from(BLS_KEYSTORE_PATH);
	let key = hex::encode(public);
	buf.push(key.as_str());
	buf
}

/// Get the key phrase for a given public key and key type.
#[cfg(feature = "std")]
fn key_phrase_by_type(public: &[u8]) -> Result<Option<String>, Error> {
	let path = key_file_path(public);

	if path.exists() {
		let file = File::open(path)?;

		serde_json::from_reader(&file).map_err(Into::into).map(Some)
	} else {
		Ok(None)
	}
}
