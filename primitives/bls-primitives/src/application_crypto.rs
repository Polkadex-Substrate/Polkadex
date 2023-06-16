// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#[cfg(feature = "std")]
pub use app::Pair as AppPair;
pub use app::{Public as AppPublic, Signature as AppSignature};
use sp_application_crypto::{KeyTypeId, RuntimePublic};
use sp_std::vec::Vec;

pub use crate::*;

pub mod app {
	use sp_core::crypto::KeyTypeId;

	pub const BLS: KeyTypeId = KeyTypeId(*b"blsk");

	sp_application_crypto::app_crypto!(super, BLS);

	// impl sp_application_crypto::BoundToRuntimeAppPublic for Public {
	// 	type Public = Self;
	// }
}

impl RuntimePublic for Public {
	type Signature = Signature;

	fn all(_: KeyTypeId) -> Vec<Self> {
		unimplemented!(
			"BLS12-381 Host functions are not yet available in Polkadot,\
		 so this will not work"
		)
	}

	#[cfg(not(feature = "parachain"))]
	fn generate_pair(key: KeyTypeId, seed: Option<Vec<u8>>) -> Self {
		crate::host_functions::bls_crypto_ext::bls_generate_pair(key, seed)
	}

	#[cfg(feature = "parachain")]
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
