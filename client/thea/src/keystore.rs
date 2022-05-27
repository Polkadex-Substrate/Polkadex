// Copyright (C) 2020-2022 Polkadex OU
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// This is file is modified from beefy-gadget from Parity Technologies (UK) Ltd.

use std::convert::{From, TryInto};

use sp_application_crypto::{Public, RuntimeAppPublic};
use sp_core::{keccak_256, Pair};
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};

use log::warn;

use thea_primitives::{
	crypto::{Public as TheaPublic, Signature},
	KEY_TYPE,
};

use crate::error;

#[cfg(test)]
#[path = "keystore_tests.rs"]
pub mod tests;

/// A Thea specific keystore implemented as a `Newtype`. This is basically a
/// wrapper around [`sp_keystore::SyncCryptoStore`] and allows to customize
/// common cryptographic functionality.
#[derive(Clone)]
pub struct TheaKeystore(Option<SyncCryptoStorePtr>);

impl TheaKeystore {
	/// Check if the keystore contains a private key for one of the public keys
	/// contained in `keys`. A public key with a matching private key is known
	/// as a local authority id.
	///
	/// Return the public key for which we also do have a private key. If no
	/// matching private key is found, `None` will be returned.
	pub fn authority_id(&self, keys: &[TheaPublic]) -> Option<TheaPublic> {
		let store = self.0.clone()?;

		// we do check for multiple private keys as a key store sanity check.
		let public: Vec<TheaPublic> = keys
			.iter()
			.filter(|k| SyncCryptoStore::has_keys(&*store, &[(k.to_raw_vec(), KEY_TYPE)]))
			.cloned()
			.collect();

		if public.len() > 1 {
			warn!(target: "beefy", "🥩 Multiple private keys found for: {:?} ({})", public, public.len());
		}

		public.get(0).cloned()
	}

	/// Sign `message` with the `public` key.
	///
	/// Note that `message` usually will be pre-hashed before being signed.
	///
	/// Return the message signature or an error in case of failure.
	pub fn sign(&self, public: &TheaPublic, message: &[u8]) -> Result<Signature, error::Error> {
		let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".into()))?;

		let msg = keccak_256(message);
		let public = public.to_public_crypto_pair();

		let sig = SyncCryptoStore::sign_with(&*store, KEY_TYPE, &public, &msg)
			.map_err(|e| error::Error::Keystore(e.to_string()))?
			.ok_or_else(|| error::Error::Signature("sign_with() failed".to_string()))?;

		// check that `sig` has the expected result type
		let sig = sig.clone().try_into().map_err(|_| {
			error::Error::Signature(format!("invalid signature {:?} for key {:?}", sig, public))
		})?;

		Ok(sig)
	}

	/// Returns a vector of [`thea_primitives::crypto::Public`] keys which are currently supported
	/// (i.e. found in the keystore).
	pub fn public_keys(&self) -> Result<Vec<TheaPublic>, error::Error> {
		let store = self.0.clone().ok_or_else(|| error::Error::Keystore("no Keystore".into()))?;

		let pk: Vec<TheaPublic> = SyncCryptoStore::sr25519_public_keys(&*store, KEY_TYPE)
			.iter()
			.map(|k| TheaPublic::from(*k))
			.collect();

		Ok(pk)
	}

	/// Use the `public` key to verify that `sig` is a valid signature for `message`.
	///
	/// Return `true` if the signature is authentic, `false` otherwise.
	pub fn verify(public: &TheaPublic, sig: &Signature, message: &[u8]) -> bool {
		let msg = keccak_256(message);
		let sig = sig.as_ref();
		let public = public.as_ref();

		sp_core::sr25519::Pair::verify(sig, &msg, public)
	}
}

impl From<Option<SyncCryptoStorePtr>> for TheaKeystore {
	fn from(store: Option<SyncCryptoStorePtr>) -> TheaKeystore {
		TheaKeystore(store)
	}
}
