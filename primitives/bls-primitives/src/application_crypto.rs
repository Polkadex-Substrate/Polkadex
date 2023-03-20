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
	#[test]
	pub fn test_generate_and_load_back() {
		use super::*;
		let loaded_keys = Public::all(KeyTypeId(*b"blsk"));
		assert_eq!(loaded_keys.len(), 0);
		let public = Public::generate_pair(KeyTypeId(*b"blsk"), None);
		let loaded_keys = Public::all(KeyTypeId(*b"blsk"));
		assert_eq!(loaded_keys.len(), 1);
		let key = loaded_keys[0];
		println!("{key:?}");
		let key = AppPublic::from(key);
		println!("{key:?}");
		assert_eq!(loaded_keys[0], public);
	}
}
