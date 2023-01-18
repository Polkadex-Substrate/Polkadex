#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
use blst::min_sig::*;
#[cfg(feature = "std")]
use blst::BLST_ERROR;
use sp_runtime_interface::runtime_interface;

use sp_std::{vec, vec::Vec};

use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use xcm::latest::{MultiAsset, MultiLocation};

#[runtime_interface]
pub trait TheaExt {
	fn bls_verify(
		agg_sig: &[u8; 96],
		bit_map: u128,
		payload: &[u8],
		bls_public_keys: &[BLSPublicKey],
	) -> bool {
		let recon_sig = Signature::from_bytes(agg_sig).unwrap();
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
				let mut new_agg_pk = agg_pk.unwrap();
				new_agg_pk.add_public_key(&bls_key, false).unwrap();
				agg_pk = Some(new_agg_pk);
			}
		}
		// Generate Aggregate Signature
		let mut _agg_sig = AggregateSignature::from_signature(&recon_sig);
		let dst = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";
		let err = recon_sig.fast_aggregate_verify_pre_aggregated(
			false,
			payload,
			dst,
			&agg_pk.unwrap().to_public_key(),
		);
		if err == BLST_ERROR::BLST_SUCCESS {
			return true
		}
		false
	}
}

#[derive(Debug, Encode, Decode, PartialEq, TypeInfo, MaxEncodedLen)]
pub struct BLSPublicKey(pub [u8; 192]);

#[allow(clippy::all)]
#[derive(Encode, Decode, Clone, TypeInfo, PartialEq, Debug)]
pub enum SoloChainMessages<AccountId> {
	///(network_id:u8, who:AccountId, tx_hash: H256, asset_id: u128, amount: u128, deposit_nonce:
	/// u32)
	Deposit(u8, AccountId, sp_core::H256, u128, u128, u32),
	///(recipient: MultiLocation, Asset&Amount: MultiAsset, deposit_nonce: u32, transaction_hash:
	/// H256)
	ParachainDeposit(MultiLocation, MultiAsset, u32, sp_core::H256),
}

impl<AccountId> SoloChainMessages<AccountId> {
	pub fn get_nonce(&self) -> u32 {
		match self {
			SoloChainMessages::Deposit(_, _, _, _, _, nonce) => *nonce,
			SoloChainMessages::ParachainDeposit(_, _, nonce, _) => *nonce,
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
