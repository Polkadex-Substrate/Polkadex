mod session_keys {
	use frame_support::{pallet_prelude::Weight, traits::StorageVersion};
	use sp_runtime::impl_opaque_keys;

	use polkadex_primitives::AccountId;

	impl_opaque_keys! {
		pub struct SessionKeysV4 {
			pub grandpa: Grandpa,
			pub babe: Babe,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
		}
	}

	impl_opaque_keys! {
		pub struct SessionKeysV5 {
			pub grandpa: Grandpa,
			pub babe: Babe,
			pub im_online: ImOnline,
			pub authority_discovery: AuthorityDiscovery,
			pub orderbook: OCEX,
			pub thea: Thea,
		}
	}
	pub struct MigrateToV5<T>(sp_std::marker::PhantomData<T>);

	impl<T> MigrateToV5<T> {
		fn run(_: StorageVersion) -> Weight {
			if let Err(_) = pallet_session::QueuedKeys::<Runtime>::translate::<
				Vec<(AccountId, SessionKeysV4)>,
				_,
			>(|keys| {
				let mut new_keys: Vec<(AccountId, SessionKeysV5)> = Vec::new();
				if let Some(keys) = keys {
					for (validator, keys) in keys {
						new_keys.push((
							validator,
							SessionKeysV5 {
								grandpa: keys.grandpa,
								babe: keys.babe,
								im_online: keys.im_online,
								authority_discovery: keys.authority_discovery,
								orderbook: [0u8; 96].into(), // Set empty public key
								thea: [0u8; 96].into(),
							},
						))
					}
				}
				Some(new_keys)
			}) {
				log::error!(target:"migration","Storage type cannot be intepreted as the Vec<(AccountId, SessionKeysV4)>")
			}
			Weight::zero()
		}
	}
}
