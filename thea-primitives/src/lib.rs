#![cfg_attr(not(feature = "std"), no_std)]

use sp_runtime_interface::runtime_interface;
#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;


use sp_std::vec::Vec;

use parity_scale_codec::{Encode, Decode, MaxEncodedLen};
use scale_info::TypeInfo;

#[runtime_interface]
pub trait TheaExt {
    fn foo(agg_sig: [u8; 96], bit_map: u128, payload: Vec<u8>, bls_public_keys: Vec<BLSPublicKey>) -> bool {
        let recon_sig = Signature::from_bytes(&agg_sig).unwrap();
        let bit_map_vec = bit_map.to_be_bytes().to_vec();
        let mut signed_public_keys: Vec<PublicKey> = vec![];
        let mut agg_pk: Option<AggregatePublicKey> = None;
        for x in 0..bls_public_keys.len(){
            if bit_map_vec[x] == 1 {
                // Fetch public key
                let current_public_key = &bls_public_keys[x];
                // Create public key from Vec from bytes
                let bls_key = PublicKey::from_bytes(&bls_public_keys[x].0).unwrap();
                // Add Public key to the already aggregated public key
                if agg_pk.is_none() {
                    agg_pk = match AggregatePublicKey::aggregate(&[&bls_key], false) {
                        Ok(agg_pk) => Some(agg_pk),
                        Err(err) => panic!("Unable to create Aggregate Public KEy"),
                    };
                } else {
                    let mut new_agg_pk = agg_pk.unwrap();
                    new_agg_pk.add_public_key(&bls_key, false).unwrap();
                    agg_pk = Some(new_agg_pk);
                }
            }
        }
        // Generate Aggregate Signature
        let mut agg_sig = AggregateSignature::from_signature(&recon_sig);
        let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
        let err = recon_sig.fast_aggregate_verify_pre_aggregated(
            false,
            &payload,
            dst,
            &agg_pk.unwrap().to_public_key(),
        );
        if err == BLST_ERROR::BLST_SUCCESS {
            return true;
        }
        false
    }
}

#[derive(Debug, Encode, Decode, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct BLSPublicKey([u8; 192]);