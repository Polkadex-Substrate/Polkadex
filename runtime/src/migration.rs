pub mod session_keys {
	use frame_support::{pallet_prelude::Weight, traits::OnRuntimeUpgrade};
	use sp_runtime::impl_opaque_keys;
	use sp_std::vec::Vec;

	use crate::{AuthorityDiscovery, Babe, Grandpa, ImOnline, Runtime, SessionKeys, Thea};
	use bls_primitives::application_crypto::app::Public as BLSPublic;
	use pallet_ocex_lmp::sr25519::AuthorityId as OCEXId;
	use polkadex_primitives::AccountId;

	impl_opaque_keys! {
		pub struct SessionKeysV5 {
			pub grandpa: Grandpa,
			pub babe: Babe,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
			pub orderbook: BLSPublic,
			pub thea: Thea,
		}
	}

	pub struct MigrateToV5;

	impl MigrateToV5 {
		fn run() -> Weight {
			// Clear Snapshot Nonce
			pallet_ocex_lmp::SnapshotNonce::<Runtime>::kill();

			// Clear Snapshots Storage
			let mut cursor = None;
			#[allow(unused_assignments)]
			let mut key_long_enough = Vec::new();
			loop {
				let results = pallet_ocex_lmp::Snapshots::<Runtime>::clear(50, cursor);

				match results.maybe_cursor {
					None => break,
					Some(key) => {
						key_long_enough = key;
						cursor = Some(key_long_enough.as_ref())
					}
				}
			}

			let translate_fn = |keys: Option<Vec<(AccountId, SessionKeysV5)>>| {
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
								orderbook: OCEXId::from(sp_core::sr25519::Public::from_raw(
									[0u8; 32],
								)),
								thea: keys.thea,
							},
						);
						log::info!(target:"migration","Migrated session key: {:?}",new_key);
						new_keys.push(new_key);
					}
				}
				Some(new_keys)
			};

			if pallet_session::QueuedKeys::<Runtime>::translate::<
				Vec<(AccountId, SessionKeysV5)>,
				_,
			>(translate_fn).is_err() {
				log::error!(target:"migration","Storage type cannot be interpreted as the Vec<(AccountId, SessionKeysV4)>")
			}

			pallet_session::NextKeys::<Runtime>::translate::<SessionKeysV5, _>(|_, old_keys| {
				Some(SessionKeys {
					grandpa: old_keys.grandpa,
					babe: old_keys.babe,
					im_online: old_keys.im_online,
					authority_discovery: old_keys.authority_discovery,
					orderbook: OCEXId::from(sp_core::sr25519::Public::from_raw([0u8; 32])),
					thea: old_keys.thea,
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
