pub mod session_keys {
	use frame_support::{pallet_prelude::Weight, traits::StorageVersion};
	use frame_support::traits::OnRuntimeUpgrade;
	use log::log;
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
		fn run(_: StorageVersion) -> Weight {
			if let Err(_) = pallet_session::QueuedKeys::<Runtime>::translate::<
				Vec<(AccountId, SessionKeysV4)>,
				_,
			>(|keys| {
				let mut new_keys: Vec<(AccountId, SessionKeys)> = Vec::new();
				if let Some(keys) = keys {
					for (validator, keys) in keys {
						new_keys.push((
							validator,
							SessionKeys {
								grandpa: keys.grandpa,
								babe: keys.babe,
								im_online: keys.im_online,
								authority_discovery: keys.authority_discovery,
								orderbook: match [0u8; 96].as_ref().try_into() {
									Ok(ob) => ob,
									Err(_) => return None,
								}, // Set empty public key
								thea:  match [0u8; 96].as_ref().try_into() {
									Ok(thea) => thea,
									Err(_) => return None,
								},
							},
						))
					}
				}
				Some(new_keys)
			}) {
				log::error!(target:"migration","Storage type cannot be interpreted as the Vec<(AccountId, SessionKeysV4)>")
			}
			Weight::zero()
		}
	}

	impl OnRuntimeUpgrade for MigrateToV2 {
		fn on_runtime_upgrade() -> Weight {
			let current = pallet_session::<Runtime>::current_storage_version();
			let onchain = pallet_session::<Runtime>::on_chain_storage_version();

			log!(
				info,
				"Running migration with current storage version {:?} / onchain {:?}",
				current,
				onchain
			);

			Self::run(current)
		}

		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
			Ok(Vec::new())
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(_: Vec<u8>) -> Result<(), &'static str> {
			let session_keys: Vec<(AccountId, SessionKeys)> = pallet_session::QueuedKeys::<Runtime>::get();
			if session_keys.is_empty() {
				return Err("Error reading sessiong keys after upgrade")
			}
			Ok(())
		}
	}
}
