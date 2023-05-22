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
		unimplemented!()
	}

	fn generate_pair(_: KeyTypeId, _: Option<Vec<u8>>) -> Self {
		// NOTE: this is just to make --dev mode compile, it will not work
		Public([0; 96])
	}

	fn sign<M: AsRef<[u8]>>(&self, _: KeyTypeId, _: &M) -> Option<Self::Signature> {
		unimplemented!()
	}

	fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool {
		signature.verify(&[*self], msg.as_ref())
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		self.0.to_vec()
	}
}
