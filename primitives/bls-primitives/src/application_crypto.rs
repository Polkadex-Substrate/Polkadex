pub use crate::*;
use sp_application_crypto::{KeyTypeId, RuntimePublic};
use sp_std::vec::Vec;

pub mod app {
	use sp_core::crypto::KeyTypeId;

	pub const BLS: KeyTypeId = KeyTypeId(*b"blsk");

	sp_application_crypto::app_crypto!(super, BLS);

	impl sp_application_crypto::BoundToRuntimeAppPublic for Public {
		type Public = Self;
	}
}

#[cfg(feature = "std")]
pub use app::Pair as AppPair;
pub use app::{Public as AppPublic, Signature as AppSignature};

impl RuntimePublic for Public {
	type Signature = Signature;

	fn all(_: KeyTypeId) -> Vec<Self> {
		crypto::bls_ext::all()
	}

	fn generate_pair(_: KeyTypeId, seed: Option<Vec<u8>>) -> Self {
		crypto::bls_ext::generate_pair_and_store(seed)
	}

	fn sign<M: AsRef<[u8]>>(&self, _: KeyTypeId, msg: &M) -> Option<Self::Signature> {
		crypto::bls_ext::sign(self, msg.as_ref())
	}

	fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool {
		crypto::bls_ext::verify(self, msg.as_ref(), signature)
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		self.0.to_vec()
	}
}

#[cfg(test)]
mod tests {
	use crate::crypto::sign;
	use sp_core::blake2_256;

	#[test]
	pub fn test_generate_and_load_back() {
		use super::*;
		let key_type = KeyTypeId(*b"blsk");
		let public = Public::generate_pair(key_type, Some(b"owner word vocal dose decline sunset battle example forget excite gentle waste//1//orderbook".to_vec()));
		let loaded_keys = Public::all(key_type);
		assert_eq!(loaded_keys.len(), 1);
		assert_eq!(loaded_keys[0], public);
		let message = blake2_256(&vec![0, 1]);

		let signature = sign(&public, &message).unwrap();
		println!("Pubkey: {:?}", public.0);
		println!("Signature: {:?}", signature.0);
		assert!(crate::crypto::bls_ext::verify(&public, message.as_ref(), &signature));
	}
}
