use polkadex_primitives::Signature;
use sp_core::{
	crypto::{AccountId32, ByteArray},
	ed25519, sr25519,
};
use sp_runtime::{
	traits::{Lazy, Verify},
	MultiSigner,
};
/// In this custom implementation we are modifying the ecdsa verification to avoid blake256 hashing
/// the message before recovering pubk and it will fail if the message size is greater or less than
/// 32.
pub struct CustomSignature {
	pub(crate) signature: Signature,
}

impl Verify for CustomSignature {
	type Signer = MultiSigner;

	fn verify<L: Lazy<[u8]>>(&self, mut msg: L, signer: &AccountId32) -> bool {
		match (&self.signature, signer) {
			(Signature::Ed25519(ref sig), who) => match ed25519::Public::from_slice(who.as_ref()) {
				Ok(signer) => sig.verify(msg, &signer),
				Err(()) => {
					log::error!(target:"signature-verification", "Failed to verify Ed25519 signature");
					false
				},
			},
			(Signature::Sr25519(ref sig), who) => match sr25519::Public::from_slice(who.as_ref()) {
				Ok(signer) => sig.verify(msg, &signer),
				Err(()) => {
					log::error!(target:"signature-verification", "Failed to verify Sr25519 signature");
					false
				},
			},
			(Signature::Ecdsa(ref sig), who) => match msg.get().try_into() {
				Err(_) => {
					log::error!(target:"signature-verification", "Failed to verify Ecdsa signature");
					false
				},
				Ok(m) =>
					match sp_io::crypto::secp256k1_ecdsa_recover_compressed(sig.as_ref(), &m) {
						Ok(pubkey) =>
							&sp_io::hashing::blake2_256(pubkey.as_ref()) ==
								<dyn AsRef<[u8; 32]>>::as_ref(who),
						_ => false,
					},
			},
		}
	}
}
