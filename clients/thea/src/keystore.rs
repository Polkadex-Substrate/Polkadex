use crate::error::Error;
use log::warn;
use sc_keystore::LocalKeystore;
use sp_core::Pair;
use sp_keystore::SyncCryptoStore;
use std::sync::Arc;
use thea_primitives::{
	crypto::{AuthorityId, AuthoritySignature},
	KEY_TYPE,
};

pub struct TheaKeyStore {
	keystore: Option<Arc<LocalKeystore>>,
}

impl TheaKeyStore {
	pub fn new(keystore: Option<Arc<LocalKeystore>>) -> Self {
		Self { keystore }
	}

	pub fn get_local_key(&self, active: &[AuthorityId]) -> Result<AuthorityId, Error> {
		match self.keystore.as_ref() {
			None => {
				warn!(target:"thea","Keystore not available");
				return Err(Error::Keystore("Keystore not available in this context".to_string()))
			},
			Some(keystore) =>
				for key in active {
					if let Some(local_pair) =
						keystore.key_pair::<thea_primitives::crypto::Pair>(key)?
					{
						return Ok(local_pair.public())
					}
				},
		}
		warn!(target:"thea","No BLS key found");
		Err(Error::Keystore("No BLS key found".to_string()))
	}

	pub fn sign(&self, public: &AuthorityId, message: &[u8]) -> Result<AuthoritySignature, Error> {
		match self.keystore.as_ref() {
			None => {
				warn!(target:"thea","Keystore not available");
				Err(Error::Keystore("Keystore not available in this context".to_string()))
			},
			Some(keystore) => match keystore.key_pair::<thea_primitives::crypto::Pair>(public)? {
				Some(local_pair) => Ok(local_pair.sign(message)),
				None => {
					warn!(target:"thea","No BLS key found");
					Err(Error::Keystore("No BLS key found".to_string()))
				},
			},
		}
	}

	pub fn public_keys(&self) -> Vec<sp_core::sr25519::Public> {
		let store = self.keystore.clone().unwrap();
		SyncCryptoStore::sr25519_public_keys(&*store, KEY_TYPE)
			.iter()
			.map(|k| *k)
			.collect()
	}
}
