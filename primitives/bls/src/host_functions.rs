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

// DISCLAIMER: This module is deprecated and exists solely for the host function required during block sync.
// It will not be maintained and must not be used in production.

use sp_application_crypto::RuntimePublic;
use sp_core::crypto::KeyTypeId;
#[cfg(feature = "std")]
use sp_core::Pair;
#[cfg(feature = "std")]
use sp_keystore::{Keystore, KeystoreExt};
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
                let (pair, seed_string, _) = crate::Pair::generate_with_phrase(None);
                (pair, seed_string)
            },
            Some(seed) => {
                let seed = String::from_utf8(seed).expect("expected seed to be Utf-8");
                (crate::Pair::from_string(seed.as_str(), None).expect("Seed not valid!"), seed)
            },
        };
        let keystore = &***self
            .extension::<KeystoreExt>()
            .expect("No `keystore` associated for the current context!");
        let public_key = pair.public().to_raw_vec();
        <(dyn Keystore + 'static)>::insert(keystore, id, seed.as_str(), public_key.as_slice())
            .unwrap();
        pair.public()
    }
}