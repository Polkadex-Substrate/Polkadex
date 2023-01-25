#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;
use sp_runtime_interface::runtime_interface;

use sp_std::{vec, vec::Vec};

use crate::parachain_primitives::ParachainWithdraw;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

pub mod normal_deposit;
pub mod parachain_primitives;
pub mod thea_types;

#[runtime_interface]
pub trait TheaExt {
	fn bls_verify(
		agg_sig: &[u8; 96],
		bit_map: u128,
		payload: &[u8],
		bls_public_keys: &[BLSPublicKey],
	) -> bool {
		let recon_sig = match Signature::from_bytes(agg_sig) {
			Ok(sig) => sig,
			Err(_e) => {
				return false
			},
		};
		let bit_map_vec = return_set_bits(bit_map);
		let mut agg_pk: Option<AggregatePublicKey> = None;
		for x in bit_map_vec {
			// Fetch public key
			let _current_public_key = &bls_public_keys[x as usize];
			// Create public key from Vec from bytes
			let bls_key = PublicKey::from_bytes(&bls_public_keys[x as usize].0).unwrap();
			// Add Public key to the already aggregated public key
			if agg_pk.is_none() {
				agg_pk = match AggregatePublicKey::aggregate(&[&bls_key], false) {
					Ok(agg_pk) => Some(agg_pk),
					Err(_err) => return false,
				};
			} else {
				if let Some(mut new_agg_pk) = agg_pk {
					new_agg_pk.add_public_key(&bls_key, false).unwrap();
					agg_pk = Some(new_agg_pk);
				} else {
					return false
				}
			}
		}
		// Generate Aggregate Signature
		let mut _agg_sig = AggregateSignature::from_signature(&recon_sig);
		let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
		if let Some(agg_pk) = agg_pk {
			let err = recon_sig.fast_aggregate_verify_pre_aggregated(
				false,
				payload,
				dst,
				&agg_pk.to_public_key(),
			);
			if err == BLST_ERROR::BLST_SUCCESS {
				return true
			}
		}
		false
	}
}

#[derive(
	Clone, Debug, Encode, Decode, PartialEq, TypeInfo, MaxEncodedLen, PartialOrd, Ord, Eq, Copy,
)]
pub struct BLSPublicKey(pub [u8; 192]);

#[derive(Encode, Decode, Clone, Debug, TypeInfo, Eq, PartialEq)]
pub struct ApprovedWithdraw {
	pub asset_id: u128,
	pub amount: u128,
	pub network: u8,
	pub beneficiary: Vec<u8>,
	pub payload: Vec<u8>,
}

impl ApprovedWithdraw {
	pub fn decode_payload(&self) -> Option<ParachainWithdraw> {
		ParachainWithdraw::decode(&mut &self.payload[..]).ok()
	}
}

pub trait AssetIdConverter {
	/// Get Asset Id
	fn get_asset_id(&self) -> Option<u128>;

	/// To Asset Id
	fn to_asset_id(&self) -> Self;
}

#[derive(Debug, Encode, Decode, Clone, PartialEq, TypeInfo, MaxEncodedLen)]
pub enum TokenType {
	Fungible(u8),
	NonFungible(u8),
	Generic(u8),
}

impl TokenType {
	pub fn get_network_id(&self) -> u8 {
		match self {
			TokenType::Fungible(network_id) => *network_id,
			TokenType::NonFungible(network_id) => *network_id,
			TokenType::Generic(network_id) => *network_id,
		}
	}
}

pub fn return_set_bits(bit_map: u128) -> Vec<u8> {
	let mut set_bits: Vec<u8> = vec![];
	for i in 0..128 {
		if (bit_map & 2_u128.pow(i as u32)) == 2_u128.pow(i as u32) {
			set_bits.push(i as u8);
		}
	}
	set_bits
}

#[test]
pub fn test_bit_manipulation() {
	let x = 3;
	let mut set_bits: Vec<u8> = vec![];
	let mut i: usize = 0;
	while i < 128 {
		if (x & 2_u128.pow(i as u32)) == 2_u128.pow(i as u32) {
			set_bits.push(i as u8);
		}
		i += 1;
	}
	assert_eq!(set_bits, vec![0, 1]);
}

#[test]
pub fn test_set_bit_map() {
	let x: u128 = 2;
	// Set 0th bit
	let new_x = (1 << 0) | x;
	assert_eq!(new_x, 3);
}
