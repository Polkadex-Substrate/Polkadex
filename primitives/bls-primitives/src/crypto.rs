#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;
#[cfg(feature = "std")]
use sp_application_crypto::serde::Serialize;
use sp_core::crypto::KeyTypeId;
use sp_runtime_interface::runtime_interface;
use crate::{DST, Pair as BLSPair, Error, Public, Signature, Seed};
use sp_core::Pair;

use crate::application_crypto::BLSSignature;

pub const BLS_KEYSTORE_PATH: &str = "/polkadex/.keystore/";

#[runtime_interface]
pub trait BlsExt {
    fn all() -> Vec<Public>{
        // Load all available bls public keys from filesystem
        match get_all_public_keys() {
            Ok(keys) => keys,
            Err(_) => Vec::new()
        }
    }

    fn generate_pair(seed: Option<Vec<u8>>) -> Public {
        // generate the private key  and store it in filesystem
        let (pair, seed) = match seed {
            None => BLSPair::generate(),
            Some(seed) => {
                let seed = Seed::try_from(seed)
                    .expect("Expected to BLS seed to be 32 bytes");

                (BLSPair::from_seed(&seed),seed)
            }
        };
        pair.public
    }

    fn sign(pubkey: &Public, msg: &[u8]) -> Option<Signature> {
        // load the private key from filesystem and sign with it
        sign(pubkey,msg)
    }

    fn verify(pubkey: &Public, msg: &[u8], signature: &Signature) -> bool {
        let pubkey = match PublicKey::from_bytes(pubkey.0.as_ref()) {
            Ok(pubkey) => pubkey,
            Err(_) => return false
        };

        let signature = match BLSSignature::from_bytes(signature.0.as_ref()) {
            Ok(sig) => sig,
            Err(_) => return false
        };
        // verify the signature
        let err = signature
            .verify(true,msg,DST.as_ref(),&[],&pubkey,true);
        return if err == BLST_ERROR::BLST_SUCCESS {
            true
        }else{
            false
        };
    }

    fn verify_aggregate(pubkey: &Public, msg: &[u8], signature: &Signature) -> bool {
        let agg_pubkey = match PublicKey::from_bytes(pubkey.0.as_ref()) {
            Ok(pubkey) => pubkey,
            Err(_) => return false
        };

        let agg_signature = match BLSSignature::from_bytes(signature.0.as_ref()) {
            Ok(sig) => sig,
            Err(_) => return false
        };
        // verify the signature
        let err = agg_signature
            .fast_aggregate_verify_pre_aggregated(true,msg,DST.as_ref(),&agg_pubkey);
        return if err == BLST_ERROR::BLST_SUCCESS {
            true
        }else{
            false
        };
    }
}

#[cfg(feature = "std")]
use std::fs::File;
#[cfg(feature = "std")]
use std::io::Write;
#[cfg(feature = "std")]
use std::os::unix::fs;
#[cfg(feature = "std")]
use std::path::PathBuf;

#[cfg(feature = "std")]
fn sign(pubkey: &Public, msg: &[u8]) -> Option<Signature> {
    let path = key_file_path(pubkey.as_ref());
    match std::fs::read(&path){
        Err(_) => return None,
        Ok(seed) => {
            match SecretKey::from_bytes(&seed) {
                Ok(secret_key) => {
                    Some(Signature::from(secret_key.sign(msg,DST.as_ref(),&[])))
                }
                Err(_) => return None
            }
        }
    }
}

#[cfg(feature = "std")]
fn get_all_public_keys() -> Result<Vec<Public>,Error>{
    let mut public_keys = vec![];
    for entry in std::fs::read_dir(&BLS_KEYSTORE_PATH)? {
        let entry = entry?;
        let path = entry.path();

        // skip directories and non-unicode file names (hex is unicode)
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            match hex::decode(name) {
                Ok(ref hex) if hex.len() > 4 => {
                    let public = hex.to_vec();
                    match Public::try_from(public.as_ref()){
                        Ok(public) => public_keys.push(public),
                        Err(_) => continue
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
