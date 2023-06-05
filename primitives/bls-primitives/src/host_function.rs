use sp_application_crypto::RuntimePublic;
use sp_core::crypto::KeyTypeId;
#[cfg(feature = "std")]
use sp_core::Pair;
#[cfg(feature = "std")]
use sp_keystore::{KeystoreExt, SyncCryptoStore};
use sp_std::vec::Vec;

use crate::Public;
use sp_runtime_interface::runtime_interface;

#[cfg(feature = "std")]
use sp_externalities::ExternalitiesExt;

#[runtime_interface]
pub trait BLSCryptoExt {
	fn bls_generate_pair(&mut self, id: KeyTypeId, seed: Option<Vec<u8>>) -> Public {
		let (pair, seed) = match seed {
			None => {
				let (pair, seed) = crate::Pair::generate();
				(pair, seed.to_vec())
			},
			Some(seed) =>
				(crate::Pair::from_seed_slice(seed.as_slice()).expect("Seed not valid!"), seed),
		};
		let keystore = &***self
			.extension::<KeystoreExt>()
			.expect("No `keystore` associated for the current context!");
		let public_key = pair.public().to_raw_vec();
		SyncCryptoStore::insert_unknown(
			keystore,
			id,
			String::from_utf8(seed).expect("Invalid seed").as_str(),
			public_key.as_slice(),
		)
		.unwrap();
		pair.public()
	}
}
