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

#[cfg(feature = "std")]
use frost_secp256k1 as frost;
use frost_secp256k1::{
	round1::{SigningCommitments, SigningNonces},
	Identifier,
};
use parity_scale_codec::{Decode, Encode};
#[cfg(feature = "std")]
use rand::thread_rng;
use sp_runtime_interface::runtime_interface;
use sp_std::collections::btree_map::BTreeMap;

#[runtime_interface]
pub trait TheaFrostExt {
	/// Returns round1 secret package and round1 package for broadcast
	fn dkg_part1(
		participant_identifier: u16,
		max_signers: u16,
		min_signers: u16,
	) -> Result<(Vec<u8>, Vec<u8>), ()> {
		let mut rng = thread_rng();
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
			Ok((round1_secret_package, round1_package)) => Ok((
				round1_secret_package.serialize().unwrap(),
				round1_package.serialize().unwrap(),
			)),
		}
	}
	/// Returns Round 2 secret and broadcast packages
	fn dkg_part2(
		round1_secret_package: &[u8],
		encoded_round1_packages_map: Vec<u8>,
	) -> Result<(Vec<u8>, Vec<u8>), ()> {
		let mut encoded_round1_packages_map = encoded_round1_packages_map.clone(); // TODO: can we not do this?
		let encoded_round1_packages: BTreeMap<[u8; 32], Vec<u8>> =
			Decode::decode(&mut &encoded_round1_packages_map[..]).map_err(|err| {
				log::error!(target:"frost","Error while scale decoding the encoded map: {:?}",err);
				()
			})?;
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


	/// Performs the third and final part of the distributed key generation protocol for the participant holding the given round2::SecretPackage, given the received round1::Packages and round2::Packages received from the other participants.
	/// It returns the KeyPackage that has the long-lived key share for the participant, and the PublicKeyPackages that has public information about all participants; both of which are required to compute FROST signatures.
	fn dkg_part3(
		round2_secret_package: &[u8],
		encoded_round1_packages_map: Vec<u8>,
		encoded_round2_packages_map: Vec<u8>,
	) -> Result<(Vec<u8>,Vec<u8>, [u8;65]), ()> {
		let mut encoded_round1_packages_map = encoded_round1_packages_map.clone(); // TODO: can we not do this?
		let encoded_round1_packages: BTreeMap<[u8; 32], Vec<u8>> =
			Decode::decode(&mut &encoded_round1_packages_map[..]).map_err(|err| {
				log::error!(target:"frost","Error while scale decoding the encoded map: {:?}",err);
				()
			})?;

		let encoded_round2_packages: BTreeMap<[u8; 32], Vec<u8>> =
			Decode::decode(&mut &encoded_round2_packages_map[..]).map_err(|err| {
				log::error!(target:"frost","Error while scale decoding the encoded map: {:?}",err);
				()
			})?;
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

		for (k, v) in encoded_round2_packages {
			round2_packages.insert(
				frost::Identifier::deserialize(&k).unwrap(),
				frost::keys::dkg::round2::Package::deserialize(&v).unwrap(),
			);
		}

		match frost::keys::dkg::part3(&round2_secret_package, &round1_packages, &round2_packages) {
			Err(err) => {
				log::error!(target:"frost","Error while DKG_3: {:?}",err);
				return Err(())
			},
			Ok((key_package, public_key_package)) => {
				Ok((key_package.serialize().unwrap(),public_key_package.serialize().unwrap(), public_key_package.verifying_key().serialize()))
			}
		}
	}

	/// Performed once by each participant selected for the signing operation.
	/// Generates the signing nonces and commitments to be used in the signing operation.
	fn nonce_commit(key_package: Vec<u8>) -> Result<(Vec<u8>, Vec<u8>), ()> {
		let mut rng = thread_rng();

		let key_package = frost::keys::KeyPackage::deserialize(&key_package).unwrap();
		let (signing_nonces, signing_commitments) =
			frost::round1::commit(key_package.signing_share(), &mut rng);
		Ok((signing_nonces.serialize().unwrap(), signing_commitments.serialize().unwrap()))
	}

	/// Performed once by each participant selected for the signing operation.
	/// Receives the message to be signed and a set of signing commitments and a set of randomizing commitments to be used in that signing operation, including that for this participant.
	/// Assumes the participant has already determined which nonce corresponds with the commitment that was assigned by the coordinator in the SigningPackage.
	fn sign(
		encoded_commitments_map: Vec<u8>,
		encoded_signing_nonce: Vec<u8>,
		key_package: Vec<u8>,
		message: [u8; 32],
	) -> Result<([u8; 32], Vec<u8>), ()> {
		// Decode Commitments map
		let mut encoded_commitments_map = encoded_commitments_map.clone(); // TODO: can we not do this?
		let encoded_commitments_map: BTreeMap<[u8; 32], Vec<u8>> =
			Decode::decode(&mut &encoded_commitments_map[..]).map_err(|err| {
				log::error!(target:"frost","Error while scale decoding the encoded map: {:?}",err);
				()
			})?;
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

	fn aggregate(
		signing_package: Vec<u8>,
		encoded_signing_shares_map: Vec<u8>,
		publickey_package: Vec<u8>,
		message: [u8; 32],
	) -> Result<([u8; 32], u8, [u8; 32], [u8; 32], [u8; 20]), ()> {
		let signing_package = frost::SigningPackage::deserialize(&signing_package).unwrap();
		let publickey_package =
			frost::keys::PublicKeyPackage::deserialize(&publickey_package).unwrap();

		let encoded_signing_shares_map: BTreeMap<[u8; 32], [u8; 32]> =
			Decode::decode(&mut &encoded_signing_shares_map[..]).map_err(|err| {
				log::error!(target:"frost","Error while scale decoding the encoded signing shares map: {:?}",err);
				()
			})?;

		let mut signing_shares: BTreeMap<Identifier, frost::round2::SignatureShare> =
			BTreeMap::new();

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
				Ok(frost::params_for_contract(
					&signature,
					&publickey_package.verifying_key(),
					message,
				))
			},
		}
	}

	fn index_to_identifier(index: u16) -> Result<[u8;16],()> {
		Ok(frost::Identifier::try_from(index).ok_or(())?.serialize())
	}

	/// Verify a the params with signature like we do in ethereum.
	fn verify_params(message: [u8;32], params: ([u8; 32], u8, [u8; 32], [u8; 32], [u8; 20])) -> bool {

	}
}
