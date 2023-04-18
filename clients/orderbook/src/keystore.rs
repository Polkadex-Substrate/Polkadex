use log::warn;
use std::sync::Arc;

use crate::error::Error;
use orderbook_primitives::crypto::{AuthorityId, AuthoritySignature};
use sc_keystore::LocalKeystore;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::Pair;

pub struct OrderbookKeyStore {
	keystore: Option<Arc<LocalKeystore>>,
}

impl OrderbookKeyStore {
	pub fn new(keystore: Option<Arc<LocalKeystore>>) -> Self {
		Self { keystore }
	}

	pub fn get_local_key(&self, active: &[AuthorityId]) -> Result<AuthorityId, Error> {
		match self.keystore.as_ref() {
			None => {
				warn!(target:"orderbook","Keystore not available");
				return Err(Error::Keystore("Keystore not available in this context".to_string()))
			},
			Some(keystore) =>
				for key in active {
					if let Some(local_pair) =
						keystore.key_pair::<orderbook_primitives::crypto::Pair>(key)?
					{
						return Ok(local_pair.public())
					}
				},
		}
		warn!(target:"orderbook","No BLS key found");
		Err(Error::Keystore("No BLS key found".to_string()))
	}

	pub fn sign(
		&self,
		public: &AuthorityId,
		message: &[u8],
	) -> Result<orderbook_primitives::crypto::AuthoritySignature, Error> {
		match self.keystore.as_ref() {
			None => {
				warn!(target:"orderbook","Keystore not available");
				Err(Error::Keystore("Keystore not available in this context".to_string()))
			},
			Some(keystore) => {
				match keystore.key_pair::<orderbook_primitives::crypto::Pair>(public)? {
					Some(local_pair) => Ok(local_pair.sign(message)),
					None => {
						warn!(target:"orderbook","No BLS key found");
						Err(Error::Keystore("No BLS key found".to_string()))
					},
				}
			},
		}
	}

	pub fn verify(
		&self,
		public_key: &AuthorityId,
		signature: &AuthoritySignature,
		message: &[u8; 32],
	) -> bool {
		public_key.verify(message, signature)
	}
}
