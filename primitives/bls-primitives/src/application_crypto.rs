pub use crate::*;
use sp_application_crypto::{KeyTypeId, RuntimePublic};
use sp_std::vec::Vec;

mod app {
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

	fn all(key_type: KeyTypeId) -> Vec<Self> {
		todo!()
	}

	fn generate_pair(key_type: KeyTypeId, seed: Option<Vec<u8>>) -> Self {
		todo!()
	}

	fn sign<M: AsRef<[u8]>>(&self, key_type: KeyTypeId, msg: &M) -> Option<Self::Signature> {
		todo!()
	}

	fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool {
		todo!()
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		todo!()
	}
}
