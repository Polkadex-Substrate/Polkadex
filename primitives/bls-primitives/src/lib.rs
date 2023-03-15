#![cfg_attr(not(feature = "std"), no_std)]

mod application_crypto;
mod crypto;

#[cfg(test)]
mod tests;

#[cfg(feature = "std")]
use bip39::{Language, Mnemonic, MnemonicType};
#[cfg(feature = "std")]
use blst::min_sig::{
	AggregatePublicKey, AggregateSignature, PublicKey, SecretKey, Signature as BLSSignature,
};
#[cfg(feature = "std")]
use blst::BLST_ERROR;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_application_crypto::RuntimePublic;
use sp_core::crypto::{
	ByteArray, CryptoType, CryptoTypeId, CryptoTypePublicPair, Derive, KeyTypeId,
};

#[cfg(feature = "std")]
use sp_core::{crypto::SecretStringError, DeriveJunction};
use sp_runtime_interface::pass_by::PassByInner;
#[cfg(feature = "std")]
use substrate_bip39::mini_secret_from_entropy;

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

#[cfg(feature = "std")]
type Seed = [u8; 32];

/// An error when deriving a key.
#[cfg(feature = "std")]
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
		Self { 0: value }
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
		Signature(value.to_bytes())
	}
}

#[cfg(feature = "std")]
impl CryptoType for Pair {
	type Pair = Pair;
}


impl From<CryptoTypePublicPair> for Public {
	fn from(value: CryptoTypePublicPair) -> Self {
		Public::try_from(value.1.as_ref())
			.expect("Expected the public key to be 96 bytes")
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
		mut phrase: &str,
		_password: Option<&str>,
	) -> Result<(Pair, Seed), SecretStringError> {
		pub const DEV_PHRASE: &str = "bottom drive obey lake curtain smoke basket hold race lonely fit walk";
		if phrase == DEV_PHRASE {
			phrase = "forget flee list will tissue myself viable sleep cover lake summer flat artefact hurry bronze salt fiber fog emotion loyal broken coach arch plastic";
		}
		Ok(Mnemonic::from_phrase(phrase, Language::English)
			.map_err(|_| SecretStringError::InvalidPhrase)
			.map(|m| {
				let seed = m.entropy();
				assert!(seed.len()>=32);
				let secret = SecretKey::key_gen(&seed, &[]).unwrap();
				let pair = Pair { public: secret.sk_to_pk().to_bytes().into(), secret };
				(pair, seed.try_into().expect("BLS Seed is expected to be 32 bytes"))
			})?)
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
		let mut master_key = SecretKey::key_gen(&seed.unwrap(),&[])?;
		for junction in path {
			let index_bytes = [
				junction.inner()[0],
				junction.inner()[1],
				junction.inner()[2],
				junction.inner()[3]];
			master_key = master_key.derive_child_eip2333(u32::from_be_bytes(index_bytes))
		}
		Ok((Pair{ public: master_key.sk_to_pk().to_bytes().into(), secret: master_key }, seed))
	}

	fn from_seed(seed: &Self::Seed) -> Self {
		let secret = SecretKey::from_bytes(seed).expect("BLS seed is expected to be at least 32 bytes");

		Pair { public: secret.sk_to_pk().to_bytes().into(), secret }
	}

	fn from_seed_slice(seed: &[u8]) -> Result<Self, SecretStringError> {
		let secret = match SecretKey::from_bytes(seed) {
			Ok(secret) => secret,
			Err(_) => return Err(SecretStringError::InvalidSeed),
		};

		Ok(Pair { public: secret.sk_to_pk().to_bytes().into(), secret })
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
