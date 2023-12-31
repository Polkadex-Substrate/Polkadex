// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
// Host functions for Thea's frost based implementation.

use frost_secp256k1 as frost;
use frost_secp256k1::{
	round1::{SigningCommitments, SigningNonces},
	Identifier,
};
use rand_chacha::ChaCha20Rng;
use parity_scale_codec::{ Encode};
use rand_chacha::rand_core::SeedableRng;
use sp_std::collections::btree_map::BTreeMap;
use crate::types::ParamsForContract;


/// Returns round1 secret package and round1 package for broadcast
pub fn dkg_part1(
	participant_identifier: u16,
	max_signers: u16,
	min_signers: u16,
) -> Result<(Vec<u8>, Vec<u8>), ()> {
	let mut rng = ChaCha20Rng::from_seed(sp_io::offchain::random_seed());
	match frost::keys::dkg::part1(
		participant_identifier.try_into().unwrap(),
		max_signers,
		min_signers,
		&mut rng,
	) {
		Err(err) => {
			log::error!(target:"frost","Error while DKG_1: {:?}",err);
			return Err(())
		},
		Ok((round1_secret_package, round1_package)) =>
			Ok((round1_secret_package.serialize().unwrap(), round1_package.serialize().unwrap())),
	}
}
/// Returns Round 2 secret and broadcast packages
pub fn dkg_part2(
	round1_secret_package: &[u8],
	encoded_round1_packages: BTreeMap<[u8; 32], Vec<u8>>,
) -> Result<(Vec<u8>, Vec<u8>), ()> {
	let secret_package =
		frost::keys::dkg::round1::SecretPackage::deserialize(round1_secret_package).unwrap();
	let mut round1_packages: BTreeMap<frost::Identifier, frost::keys::dkg::round1::Package> =
		BTreeMap::new();

	for (k, v) in encoded_round1_packages {
		round1_packages.insert(
			frost::Identifier::deserialize(&k).unwrap(),
			frost::keys::dkg::round1::Package::deserialize(&v).unwrap(),
		);
	}

	match frost::keys::dkg::part2(secret_package, &round1_packages) {
		Err(err) => {
			log::error!(target:"frost","Error while DKG_2: {:?}",err);
			return Err(())
		},
		Ok((round2_secret_package, round2_package)) => {
			let mut encoded_round2_packages: BTreeMap<[u8; 32], Vec<u8>> = BTreeMap::new();

			for (k, v) in round2_package {
				encoded_round2_packages.insert(k.serialize(), v.serialize().unwrap());
			}

			Ok((round2_secret_package.serialize().unwrap(), encoded_round2_packages.encode()))
		},
	}
}

/// Performs the third and final part of the distributed key generation protocol for the
/// participant holding the given round2::SecretPackage, given the received round1::Packages and
/// round2::Packages received from the other participants. It returns the KeyPackage that has
/// the long-lived key share for the participant, and the PublicKeyPackages that has public
/// information about all participants; both of which are required to compute FROST signatures.
pub fn dkg_part3(
	round2_secret_package: &[u8],
	encoded_round1_packages: BTreeMap<[u8; 32], Vec<u8>>,
	encoded_round2_packages: BTreeMap<[u8; 32], BTreeMap<[u8; 32], Vec<u8>>>,
) -> Result<(Vec<u8>, Vec<u8>, [u8; 65]), ()> {
	let round2_secret_package =
		frost::keys::dkg::round2::SecretPackage::deserialize(round2_secret_package).unwrap();
	let mut round1_packages: BTreeMap<frost::Identifier, frost::keys::dkg::round1::Package> =
		BTreeMap::new();

	for (k, v) in encoded_round1_packages {
		round1_packages.insert(
			frost::Identifier::deserialize(&k).unwrap(),
			frost::keys::dkg::round1::Package::deserialize(&v).unwrap(),
		);
	}

	let mut round2_packages: BTreeMap<frost::Identifier, frost::keys::dkg::round2::Package> =
		BTreeMap::new();

	// for (k, v) in encoded_round2_packages {
	// 	round2_packages.insert(
	// 		frost::Identifier::deserialize(&k).unwrap(),
	// 		frost::keys::dkg::round2::Package::deserialize(&v).unwrap(),
	// 	);
	// }

	match frost::keys::dkg::part3(&round2_secret_package, &round1_packages, &round2_packages) {
		Err(err) => {
			log::error!(target:"frost","Error while DKG_3: {:?}",err);
			return Err(())
		},
		Ok((key_package, public_key_package)) => Ok((
			key_package.serialize().unwrap(),
			public_key_package.serialize().unwrap(),
			public_key_package.verifying_key().serialize(),
		)),
	}
}

/// Performed once by each participant selected for the signing operation.
/// Generates the signing nonces and commitments to be used in the signing operation.
pub fn nonce_commit(key_package: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>), ()> {
	let mut rng = ChaCha20Rng::from_seed(sp_io::offchain::random_seed());
	let key_package = frost::keys::KeyPackage::deserialize(&key_package).unwrap();
	let (signing_nonces, signing_commitments) =
		frost::round1::commit(key_package.signing_share(), &mut rng);
	Ok((signing_nonces.serialize().unwrap(), signing_commitments.serialize().unwrap()))
}

/// Performed once by each participant selected for the signing operation.
/// Receives the message to be signed and a set of signing commitments and a set of randomizing
/// commitments to be used in that signing operation, including that for this participant.
/// Assumes the participant has already determined which nonce corresponds with the commitment
/// that was assigned by the coordinator in the SigningPackage.
pub fn sign(
	encoded_commitments_map: BTreeMap<[u8; 32], Vec<u8>>,
	encoded_signing_nonce: Vec<u8>,
	key_package: Vec<u8>,
	message: [u8; 32],
) -> Result<([u8; 32], Vec<u8>), ()> {
	let mut commitments_map: BTreeMap<Identifier, SigningCommitments> = BTreeMap::new();

	for (k, v) in encoded_commitments_map {
		commitments_map.insert(
			Identifier::deserialize(&k).unwrap(),
			SigningCommitments::deserialize(&v).unwrap(),
		);
	}

	let signing_nonce = SigningNonces::deserialize(&encoded_signing_nonce).unwrap();
	let key_package = frost::keys::KeyPackage::deserialize(&key_package).unwrap();
	let signing_package = frost::SigningPackage::new(commitments_map, &message);
	return match frost::round2::sign(&signing_package, &signing_nonce, &key_package) {
		Err(err) => {
			log::error!(target:"frost","Error while frost sign(): {:?}",err);
			Err(())
		},
		Ok(signature_share) =>
			Ok((signature_share.serialize(), signing_package.serialize().unwrap())),
	}
}

pub fn aggregate(
	signing_package: Vec<u8>,
	encoded_signing_shares_map: BTreeMap<[u8; 32], [u8; 32]>,
	publickey_package: Vec<u8>,
	message: [u8; 32],
) -> Result<ParamsForContract, ()> {
	let signing_package = frost::SigningPackage::deserialize(&signing_package).unwrap();
	let publickey_package = frost::keys::PublicKeyPackage::deserialize(&publickey_package).unwrap();

	let mut signing_shares: BTreeMap<Identifier, frost::round2::SignatureShare> = BTreeMap::new();

	for (k, v) in encoded_signing_shares_map {
		signing_shares.insert(
			Identifier::deserialize(&k).unwrap(),
			frost::round2::SignatureShare::deserialize(v).unwrap(),
		);
	}
	match frost::aggregate(&signing_package, &signing_shares, &publickey_package) {
		Err(err) => {
			log::error!(target:"frost","Error while frost signature aggregation: {:?}",err);
			return Err(())
		},
		Ok(signature) => {
			// construct the contract params
			let (a,b,c,d,e) =frost::params_for_contract(&signature, &publickey_package.verifying_key(), message);
			Ok(ParamsForContract{
				p_x: a,
				nonce_parity: b,
				signature: c,
				message: d,
				nonce_times_generator: e,
			})
		},
	}
}

pub fn index_to_identifier(index: u16) -> Result<[u8; 32], ()> {
	Ok(frost::Identifier::try_from(index).map_err(|_| ())?.serialize())
}

/// Verify a the params with signature like we do in ethereum.
pub fn verify_params(_message: [u8; 32], _params: ParamsForContract) -> bool {
	todo!()
}
