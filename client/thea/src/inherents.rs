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

use lazy_static::lazy_static;
use log::error;
use parking_lot::Mutex;
use sp_inherents::{
	InherentData, InherentDataProvider as InherentDataProviderTrait, InherentIdentifier,
};

use std::{collections::HashMap, fmt::Debug, sync::Arc};

use sp_application_crypto::ecdsa::Public;
use thea_primitives::{
	inherents::{InherentError, TheaPublicKeyInherentDataType, INHERENT_IDENTIFIER},
	ValidatorSetId, GENESIS_AUTHORITY_SET_ID,
};

lazy_static! {
	static ref INHERENT_DATA_STORAGE: Arc<Mutex<InherentDataProvider>> =
		Arc::new(Mutex::new(InherentDataProvider::new()));
}
#[derive(Debug, Clone, Default)]
pub struct InherentDataProvider {
	pub(crate) public_keys: HashMap<ValidatorSetId, sp_core::ecdsa::Public>,
	pub(crate) current_set_id: ValidatorSetId,
}

impl InherentDataProvider {
	pub fn new() -> Self {
		Self { public_keys: HashMap::new(), current_set_id: GENESIS_AUTHORITY_SET_ID }
	}

	pub fn update_shared_public_key(
		&mut self,
		set_id: ValidatorSetId,
		key: sp_core::ecdsa::Public,
	) -> Option<Public> {
		self.current_set_id = set_id;
		self.public_keys.insert(set_id, key)
	}
}

#[async_trait::async_trait]
impl InherentDataProviderTrait for InherentDataProvider {
	fn provide_inherent_data(
		&self,
		inherent_data: &mut InherentData,
	) -> Result<(), sp_inherents::Error> {
		// We can insert any data that implements [`codec::Encode`].
		if let Some(public_key) = self.public_keys.get(&self.current_set_id) {
			inherent_data.put_data(
				INHERENT_IDENTIFIER,
				&TheaPublicKeyInherentDataType {
					public_key: Some(public_key.clone()),
					set_id: self.current_set_id,
				},
			)
		} else {
			inherent_data.put_data(
				INHERENT_IDENTIFIER,
				&TheaPublicKeyInherentDataType { public_key: None, set_id: self.current_set_id },
			)
		}
	}

	/// When validating the inherents, the runtime implementation can throw errors. We support
	/// two error modes, fatal and non-fatal errors. A fatal error means that the block is invalid
	/// and this function here should return `Err(_)` to not import the block. Non-fatal errors
	/// are allowed to be handled here in this function and the function should return `Ok(())`
	/// if it could be handled. A non-fatal error is for example that a block is in the future
	/// from the point of view of the local node. In such a case the block import for example
	/// should be delayed until the block is valid.
	///
	/// If this functions returns `None`, it means that it is not responsible for this error or
	/// that the error could not be interpreted.
	async fn try_handle_error(
		&self,
		identifier: &InherentIdentifier,
		error: &[u8],
	) -> Option<Result<(), sp_inherents::Error>> {
		// Check if this error belongs to us.
		if *identifier != INHERENT_IDENTIFIER {
			return None
		}

		match InherentError::try_from(&INHERENT_IDENTIFIER, error)? {
			InherentError::InvalidPublicKey(wrong_key) => {
				if let Some(public_key) = wrong_key.public_key.clone() {
					error!(target: "thea", "Invalid Public Key: {:?} in Imported Block", public_key);
					Some(Err(sp_inherents::Error::Application(Box::from(
						InherentError::InvalidPublicKey(wrong_key),
					))))
				} else {
					error!(target: "thea", "No Public Key found in Imported Block");
					Some(Err(sp_inherents::Error::Application(Box::from(
						InherentError::InvalidPublicKey(wrong_key),
					))))
				}
			},
			InherentError::WrongInherentCall => {
				error!(target: "thea", "Invalid Call inserted in block");
				Some(Err(sp_inherents::Error::Application(Box::from(
					InherentError::WrongInherentCall,
				))))
			},
		}
	}
}

/// Returns the THEA Public key if it is available
pub fn get_thea_inherent_data() -> InherentDataProvider {
	INHERENT_DATA_STORAGE.lock().clone()
}

/// Sets the THEA public key
pub fn update_shared_public_key(
	set_id: ValidatorSetId,
	public_key: sp_core::ecdsa::Public,
) -> Option<Public> {
	INHERENT_DATA_STORAGE.lock().update_shared_public_key(set_id, public_key)
}
