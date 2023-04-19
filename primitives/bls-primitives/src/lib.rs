#![cfg_attr(not(feature = "std"), no_std)]

pub mod application_crypto;
pub mod crypto;

#[cfg(feature = "std")]
use bip39::{Language, Mnemonic, MnemonicType};
#[cfg(feature = "std")]
use blst::min_sig::{PublicKey, SecretKey, Signature as BLSSignature};
#[cfg(feature = "std")]
use blst::BLST_ERROR;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_core::crypto::{ByteArray, CryptoType, CryptoTypeId, CryptoTypePublicPair, Derive};

#[cfg(feature = "std")]
use sp_core::crypto::SecretStringError;
#[cfg(feature = "std")]
use sp_core::DeriveJunction;

use sp_runtime_interface::pass_by::PassByInner;
#[cfg(feature = "std")]
use substrate_bip39::seed_from_entropy;

use sp_std::vec::Vec;
#[cfg(feature = "std")]
use crate::crypto::add_signature_;

/// An identifier used to match public keys against bls keys
pub const CRYPTO_ID: CryptoTypeId = CryptoTypeId(*b"blss");

pub const DST: &str = "BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

pub const BLS_DEV_PHRASE: &str =
	"forget flee list will tissue myself viable sleep cover lake summer \
flat artefact hurry bronze salt fiber fog emotion loyal broken coach arch plastic";

pub const DEV_PHRASE: &str =
	"bottom drive obey lake curtain smoke basket hold race lonely fit walk";

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

// KeyStore for Storing Seed and Junctions
#[cfg_attr(feature = "std", derive(Hash))]
#[derive(Clone, Encode, Decode, Eq, PartialEq, Debug)]
pub struct KeyStore {
	seed: Seed,
	#[cfg(feature = "std")]
	junctions: Vec<DeriveJunction>,
}

#[cfg(feature = "std")]
impl KeyStore {
	fn new(seed: Seed, junctions: Vec<DeriveJunction>) -> Self {
		Self { seed, junctions }
	}

	fn get_seed(&self) -> Seed {
		self.seed
	}

	fn get_junctions(&self) -> Vec<DeriveJunction> {
		self.junctions.clone()
	}
}

impl Signature {
	// Aggregates two signatures
	#[cfg(feature = "std")]
	pub fn add_signature(&self, signature: &Signature) -> Result<Signature,()> {
		add_signature_(&self,signature)
	}
}

type Seed = [u8; 32];

/// An error when deriving a key.
#[cfg(feature = "std")]
#[derive(Debug)]
pub enum Error {
	/// Invalid Public key
	InvalidPublicKey,
	BLSError(BLST_ERROR),
	InvalidSeed,
	InvalidJunctionForDerivation,
	#[cfg(feature = "std")]
	SerdeError(serde_json::Error),
	#[cfg(feature = "std")]
	IOError(std::io::Error),
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
		if seed.is_none() {
			return Err(Error::InvalidSeed)
		}
		let mut master_key = SecretKey::key_gen(&seed.unwrap(), &[])?;
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
		println!("Seed: {:?}, len: {:?}", seed, seed.len());
		let secret = match SecretKey::key_gen(seed, &[]) {
			Ok(secret) => secret,
			Err(err) => {
				println!("BLS err: {err:?}");
				return Err(SecretStringError::InvalidSeed)
			},
		};

		Ok(Pair { public: secret.sk_to_pk().compress().into(), secret })
	}

	fn sign(&self, message: &[u8]) -> Self::Signature {
		self.secret.sign(message, DST.as_ref(), &[]).into()
	}

	fn verify<M: AsRef<[u8]>>(sig: &Self::Signature, message: M, pubkey: &Self::Public) -> bool {
		let pubkey = PublicKey::from_bytes(&pubkey.0).expect("Expected valid public key");
		let signature =
			BLSSignature::from_bytes(sig.0.as_ref()).expect("Expected valid BLS signature");

		signature.verify(
			true,
			message.as_ref(),
			DST.as_ref(),
			&[], // TODO: wtf is this?
			&pubkey,
			true,
		) == BLST_ERROR::BLST_SUCCESS
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
