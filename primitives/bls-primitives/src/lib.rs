#![cfg_attr(not(feature = "std"), no_std)]

pub mod application_crypto;

use ark_bls12_381::{
	g1::Config as G1Config, Bls12_381, G1Affine, G1Projective, G2Affine, G2Projective,
};
use ark_ec::{
	hashing::{
		curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve,
		HashToCurveError,
	},
	pairing::Pairing,
	short_weierstrass::Projective,
	AffineRepr, CurveGroup,
};
use ark_ff::{field_hashers::DefaultFieldHasher, Zero};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, SerializationError};
#[cfg(feature = "std")]
use bip39::{Language, Mnemonic, MnemonicType};
#[cfg(feature = "std")]
use blst::min_sig::{PublicKey, SecretKey, Signature as BLSSignature};
#[cfg(feature = "std")]
use blst::BLST_ERROR;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sha2::Sha256;
use sp_core::crypto::{ByteArray, CryptoType, CryptoTypeId, CryptoTypePublicPair, Derive};
use sp_std::ops::{Add, Neg};

#[cfg(feature = "std")]
use sp_core::crypto::SecretStringError;
#[cfg(feature = "std")]
use sp_core::DeriveJunction;

use sp_runtime_interface::pass_by::PassByInner;
#[cfg(feature = "std")]
use substrate_bip39::seed_from_entropy;

use sp_std::vec::Vec;

/// An identifier used to match public keys against bls keys
pub const CRYPTO_ID: CryptoTypeId = CryptoTypeId(*b"blss");

pub const DST: &str = "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

/// BLS Public Key
#[cfg_attr(feature = "std", derive(Hash))]
#[derive(
	Clone,
	Copy,
	Encode,
	Decode,
	MaxEncodedLen,
	PassByInner,
	TypeInfo,
	Eq,
	PartialEq,
	PartialOrd,
	Ord,
	Debug,
)]
pub struct Public(pub [u8; 96]);

#[cfg_attr(feature = "std", derive(Hash))]
#[derive(
	Encode, Decode, MaxEncodedLen, TypeInfo, PassByInner, PartialEq, Eq, Clone, Copy, Debug,
)]
pub struct Signature(pub [u8; 48]);

impl Signature {
	// Aggregates two signatures
	pub fn add_signature(self, signature: &Signature) -> Result<Signature, Error> {
		let sig1: G1Projective = G1Affine::deserialize_compressed(self.as_ref())?.into();
		let sig2: G1Projective = G1Affine::deserialize_compressed(signature.as_ref())?.into();
		let result: G1Projective = sig1.add(sig2);
		let mut buffer = Vec::from([0u8; 48]);
		result.serialize_compressed(buffer.as_mut_slice())?;
		if buffer.len() == 48 {
			Ok(Signature(buffer.try_into().unwrap()))
		} else {
			Err(Error::BLSSerilizationError(SerializationError::InvalidData))
		}
	}

	pub fn verify(self, public_keys: &[Public], message: &[u8]) -> bool {
		// Aggregate the public keys
		let mut g2_points = Vec::new();
		for public_key in public_keys {
			match G2Projective::deserialize_compressed(public_key.as_ref()) {
				Ok(point) => g2_points.push(point),
				Err(_) => return false,
			}
		}
		let aggregated_pubk: G2Projective = g2_points.into_iter().sum::<G2Projective>();
		// hash to curve g1
		let message = match hash_to_curve_g1(message) {
			Ok(message) => message,
			Err(_) => return false,
		};
		// Convert signature to a G1 point
		let signature: G1Affine = match G1Affine::deserialize_compressed(self.as_ref()) {
			Ok(signatyre) => signatyre,
			Err(_) => return false,
		};
		// Compute the product of pairings
		Bls12_381::multi_pairing(
			[signature, message.into_affine()],
			[G2Affine::generator().neg(), aggregated_pubk.into_affine()],
		)
		.is_zero()
	}
}

type Seed = [u8; 32];

/// An error when deriving a key.
#[derive(Debug)]
pub enum Error {
	/// Invalid Public key
	InvalidPublicKey,
	#[cfg(feature = "std")]
	BLSError(BLST_ERROR),
	InvalidSeed,
	BLSSerilizationError(SerializationError),
	InvalidJunctionForDerivation,
	#[cfg(feature = "std")]
	SerdeError(serde_json::Error),
	#[cfg(feature = "std")]
	IOError(std::io::Error),
}

impl From<SerializationError> for Error {
	fn from(value: SerializationError) -> Self {
		Self::BLSSerilizationError(value)
	}
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
	fn from(value: std::io::Error) -> Self {
		Self::IOError(value)
	}
}

#[cfg(feature = "std")]
impl From<serde_json::Error> for Error {
	fn from(value: serde_json::Error) -> Self {
		Self::SerdeError(value)
	}
}

#[cfg(feature = "std")]
impl From<BLST_ERROR> for Error {
	fn from(value: BLST_ERROR) -> Self {
		Self::BLSError(value)
	}
}

/// A key pair.
#[cfg(feature = "std")]
#[derive(Clone)]
pub struct Pair {
	public: Public,
	secret: SecretKey,
}

#[cfg(feature = "std")]
impl TryFrom<Public> for PublicKey {
	type Error = Error;

	fn try_from(value: Public) -> Result<Self, Self::Error> {
		Ok(PublicKey::from_bytes(&value.0)?)
	}
}

impl From<[u8; 96]> for Public {
	fn from(value: [u8; 96]) -> Self {
		Self(value)
	}
}

impl TryFrom<&[u8]> for Signature {
	type Error = ();

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		if value.len() != 196 {
			return Err(())
		}
		Ok(Signature(value.try_into().unwrap()))
	}
}

#[cfg(feature = "std")]
impl From<BLSSignature> for Signature {
	fn from(value: BLSSignature) -> Self {
		Signature(value.compress())
	}
}

#[cfg(feature = "std")]
impl CryptoType for Pair {
	type Pair = Pair;
}

impl From<CryptoTypePublicPair> for Public {
	fn from(value: CryptoTypePublicPair) -> Self {
		Public::try_from(value.1.as_ref()).expect("Expected the public key to be 96 bytes")
	}
}
impl ByteArray for Public {
	const LEN: usize = 96;
}

impl AsRef<[u8]> for Public {
	fn as_ref(&self) -> &[u8] {
		self.0.as_slice()
	}
}

impl AsMut<[u8]> for Public {
	fn as_mut(&mut self) -> &mut [u8] {
		self.0.as_mut()
	}
}

impl TryFrom<&[u8]> for Public {
	type Error = ();

	fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
		if value.len() != 96 {
			return Err(())
		}
		Ok(Public(value.try_into().unwrap()))
	}
}

impl Derive for Public {}

impl CryptoType for Public {
	#[cfg(feature = "std")]
	type Pair = Pair;
}

impl sp_core::crypto::Public for Public {
	fn to_public_crypto_pair(&self) -> CryptoTypePublicPair {
		CryptoTypePublicPair(CRYPTO_ID, self.0.to_vec())
	}
}

impl AsRef<[u8]> for Signature {
	fn as_ref(&self) -> &[u8] {
		self.0.as_ref()
	}
}

#[cfg(feature = "std")]
impl sp_core::crypto::Pair for Pair {
	type Public = Public;
	type Seed = Seed;
	type Signature = Signature;
	type DeriveError = Error;

	fn generate_with_phrase(password: Option<&str>) -> (Self, String, Self::Seed) {
		let mnemonic = Mnemonic::new(MnemonicType::Words24, Language::English);
		let phrase = mnemonic.phrase();
		let (pair, seed) = Self::from_phrase(phrase, password)
			.expect("All phrases generated by Mnemonic are valid; qed");
		(pair, phrase.to_owned(), seed)
	}

	fn from_phrase(
		phrase: &str,
		password: Option<&str>,
	) -> Result<(Pair, Seed), SecretStringError> {
		let big_seed = seed_from_entropy(
			Mnemonic::from_phrase(phrase, Language::English)
				.map_err(|_| SecretStringError::InvalidPhrase)?
				.entropy(),
			password.unwrap_or(""),
		)
		.map_err(|_| SecretStringError::InvalidSeed)?;
		let mut seed = Seed::default();
		seed.copy_from_slice(&big_seed[0..32]);
		let secret = SecretKey::key_gen(&seed, &[]).unwrap();
		let pair = Pair { public: secret.sk_to_pk().compress().into(), secret };
		Ok((pair, seed))
	}

	#[cfg(feature = "std")]
	fn derive<Iter: Iterator<Item = DeriveJunction>>(
		&self,
		path: Iter,
		seed: Option<Self::Seed>,
	) -> Result<(Self, Option<Self::Seed>), Self::DeriveError> {
		let mut master_key = self.secret.clone();
		for junction in path {
			let index_bytes = [
				junction.inner()[0],
				junction.inner()[1],
				junction.inner()[2],
				junction.inner()[3],
			];
			master_key = master_key.derive_child_eip2333(u32::from_be_bytes(index_bytes))
		}
		Ok((Pair { public: master_key.sk_to_pk().compress().into(), secret: master_key }, seed))
	}

	fn from_seed(seed: &Self::Seed) -> Self {
		Self::from_seed_slice(&seed[..]).expect("seed needs to be of valid length; qed")
	}

	fn from_seed_slice(seed: &[u8]) -> Result<Self, SecretStringError> {
		let secret = match SecretKey::key_gen(seed, &[]) {
			Ok(secret) => secret,
			Err(err) => {
				log::error!(target:"bls","Error while computing secret from seed: {:?}",err);
				return Err(SecretStringError::InvalidSeed)
			},
		};

		Ok(Pair { public: secret.sk_to_pk().compress().into(), secret })
	}

	fn sign(&self, message: &[u8]) -> Self::Signature {
		self.secret.sign(message, DST.as_ref(), &[]).into()
	}

	fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
		sig.verify(&[*pubkey], message.as_ref())
	}

	fn verify_weak<P: AsRef<[u8]>, M: AsRef<[u8]>>(sig: &[u8], message: M, pubkey: P) -> bool {
		match Signature::try_from(sig) {
			Ok(sig) => match Public::try_from(pubkey.as_ref()) {
				Ok(pubk) => Self::verify(&sig, message, &pubk),
				Err(_) => false,
			},
			Err(_) => false,
		}
	}

	fn public(&self) -> Self::Public {
		self.public
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		self.secret.to_bytes().to_vec()
	}
}

pub fn hash_to_curve_g1(message: &[u8]) -> Result<G1Projective, HashToCurveError> {
	let wb_to_curve_hasher = MapToCurveBasedHasher::<
		Projective<G1Config>,
		DefaultFieldHasher<Sha256, 128>,
		WBMap<G1Config>,
	>::new(DST.as_ref())?;
	Ok(wb_to_curve_hasher.hash(message)?.into())
}

#[cfg(test)]
mod tests {
	use crate::{Public, Signature, DST};
	use sp_application_crypto::RuntimePublic;
	use sp_core::Pair;

	#[test]
	pub fn test_signature_works() {
		let pair = blst::min_sig::SecretKey::key_gen(&[1u8; 32], &[]).unwrap();
		let message = b"message";
		let signature = pair.sign(message, DST.as_ref(), &[]);
		let public_key = pair.sk_to_pk();

		let new_signature: crate::Signature = Signature(signature.compress());
		let new_public_key: crate::Public = Public(public_key.compress());

		assert!(new_public_key.verify(&message, &new_signature));
		assert!(!new_public_key.verify(b"fake", &new_signature))
	}

	#[test]
	pub fn test_aggregate_signature_works() {
		let pair1 = crate::Pair::generate().0;
		let pair2 = crate::Pair::generate().0;
		let message = b"message";

		let sig1 = pair1.sign(message);
		let sig2 = pair2.sign(message);

		let aggregate_signature = sig1.add_signature(&sig2).unwrap();

		assert!(aggregate_signature.verify(&[pair1.public(), pair2.public()], message))
	}
}
