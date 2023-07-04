pub mod session_keys {
	use frame_support::{pallet_prelude::Weight, traits::OnRuntimeUpgrade};
	use sp_runtime::impl_opaque_keys;
	use sp_std::vec::Vec;

	use polkadex_primitives::AccountId;

	use crate::{AuthorityDiscovery, Babe, Grandpa, ImOnline, Runtime, SessionKeys};

	impl_opaque_keys! {
		pub struct SessionKeysV4 {
			pub grandpa: Grandpa,
			pub babe: Babe,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
		}
	}

	pub struct MigrateToV5;

	impl MigrateToV5 {
		fn run() -> Weight {
			let translate_fn = |keys: Option<Vec<(AccountId, SessionKeysV4)>>| {
				let mut new_keys: Vec<(AccountId, SessionKeys)> = Vec::new();
				if let Some(keys) = keys {
					for (validator, keys) in keys {
						let new_key = (
							validator,
							SessionKeys {
								grandpa: keys.grandpa,
								babe: keys.babe,
								im_online: keys.im_online,
								authority_discovery: keys.authority_discovery,
								thea: match [0u8; 96].as_ref().try_into() {
									Ok(thea) => thea,
									Err(_) => return None,
								},
							},
						);
						log::info!(target:"migration","Migrated session key: {:?}",new_key);
						new_keys.push(new_key);
					}
				}
				Some(new_keys)
			};

			if pallet_session::QueuedKeys::<Runtime>::translate::<
				Vec<(AccountId, SessionKeysV4)>,
				_,
			>(translate_fn).is_err() {
				log::error!(target:"migration","Storage type cannot be interpreted as the Vec<(AccountId, SessionKeysV4)>")
			}

			pallet_session::NextKeys::<Runtime>::translate::<SessionKeysV4, _>(|_, old_keys| {
				Some(SessionKeys {
					grandpa: old_keys.grandpa,
					babe: old_keys.babe,
					im_online: old_keys.im_online,
					authority_discovery: old_keys.authority_discovery,
					thea: match [0u8; 96].as_ref().try_into() {
						Ok(thea) => thea,
						Err(_) => return None,
					},
				})
			});
			Weight::zero()
		}
	}

	impl OnRuntimeUpgrade for MigrateToV5 {
		fn on_runtime_upgrade() -> Weight {
			log::info!("Running migration for session pallet's queued keys storage ");
			Self::run()
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, sp_runtime::TryRuntimeError> {
			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), sp_runtime::TryRuntimeError> {
			let session_keys: Vec<(AccountId, SessionKeys)> =
				pallet_session::QueuedKeys::<Runtime>::get();
			if session_keys.is_empty() {
				return Err(sp_runtime::TryRuntimeError::Other(
					"Error reading sessiong keys after upgrade",
				))
			}
			Ok(())
		}
	}
}
