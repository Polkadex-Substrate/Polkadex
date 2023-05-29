use sp_application_crypto::{KeyTypeId, RuntimePublic};
use sp_std::vec::Vec;

#[cfg(feature = "std")]
pub use app::Pair as AppPair;
pub use app::{Public as AppPublic, Signature as AppSignature};

pub use crate::*;

pub mod app {
	use sp_core::crypto::KeyTypeId;

	pub const BLS: KeyTypeId = KeyTypeId(*b"blsk");

	sp_application_crypto::app_crypto!(super, BLS);

	impl sp_application_crypto::BoundToRuntimeAppPublic for Public {
		type Public = Self;
	}
}

impl RuntimePublic for Public {
	type Signature = Signature;

	fn all(_: KeyTypeId) -> Vec<Self> {
		unimplemented!(
			"BLS12-381 Host functions are not yet available in Polkadot,\
		 so this will not work"
		)
	}

	fn generate_pair(_: KeyTypeId, _: Option<Vec<u8>>) -> Self {
		unimplemented!(
			"BLS12-381 Host functions are not yet available in Polkadot,\
		 so this will not work"
		)
	}

	fn sign<M: AsRef<[u8]>>(&self, _: KeyTypeId, _: &M) -> Option<Self::Signature> {
		unimplemented!(
			"BLS12-381 Host functions are not yet available in Polkadot,\
		 so this will not work"
		)
	}

	fn verify<M: AsRef<[u8]>>(&self, msg: &M, signature: &Self::Signature) -> bool {
		signature.verify(&[*self], msg.as_ref())
	}

	fn to_raw_vec(&self) -> Vec<u8> {
		self.0.to_vec()
	}
}
