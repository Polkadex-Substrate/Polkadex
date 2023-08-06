use crate::{
	pallet::{Accounts, AllowlistedToken, IngressMessages},
	storage::OffchainState,
	validator::WORKER_STATUS,
	Config, Pallet,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{AccountId, AssetId};
use rust_decimal::Decimal;
use sp_core::ByteArray;
use sp_runtime::{offchain::storage::StorageValueRef, traits::BlockNumberProvider, DispatchError, Saturating, SaturatedConversion};
use sp_std::collections::btree_map::BTreeMap;

impl<T: Config> Pallet<T> {
	/// Returns Some(()), if lock is acquired else, None.
	pub fn acquire_offchain_lock() -> Option<()> {
		// Check if another worker is already running or not
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		match s_info.get::<bool>().map_err(|err| {
			log::error!(target:"ocex","Error while loading worker status: {:?}",err);
			"Unable to load worker status"
		}).ok()? {
			Some(true) => {
				// Another worker is online, so exit
				log::info!(target:"ocex", "Another worker is online, so exit");
				return None
			},
			None => {},
			Some(false) => {},
		}
		s_info.set(&true); // Set WORKER_STATUS to true
		Some(())
	}

	/// Release offchain lock
	pub fn release_offchain_lock() {
		// Check if another worker is already running or not
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		s_info.set(&false); // Set WORKER_STATUS to true
	}

	pub fn get_balances(
		state: &mut OffchainState,
		account: &AccountId,
	) -> Result<BTreeMap<AssetId, Decimal>, &'static str> {
		match state.get(&account.to_raw_vec())? {
			None => Ok(BTreeMap::new()),
			Some(encoded) => BTreeMap::decode(&mut &encoded[..])
				.map_err(|_| "Unable to decode balances for account"),
		}
	}

	/// Calculates the deviation of all assets with Offchain and On-chain data.
	///
	/// This is a blocking call for offchain worker.
	pub fn calculate_inventory_deviation() -> Result<BTreeMap<AssetId, Decimal>, DispatchError> {
		// Acquire the lock to run off-chain worker
		if let Some(_) = Self::acquire_offchain_lock() {
			//      2. Load last processed blk
			let mut root = crate::storage::load_trie_root();
			log::info!(target:"ocex-rpc","state_root {:?}", root);
			let mut storage = crate::storage::State;
			let mut state = OffchainState::load(&mut storage, &mut root);
			let mut state_info = Self::load_state_info(&mut state)?;
			let last_processed_blk = state_info.last_block;
			//      3. Load all main accounts and registered assets from on-chain
			let mut offchain_inventory = BTreeMap::new();
			for (main, acc_info) in <Accounts<T>>::iter() {
				//      4. Compute sum of all balances of all assets
				let balances: BTreeMap<AssetId,Decimal> = Self::get_balances(&mut state, &Decode::decode(&mut &main.encode()[..]).unwrap())?;
				for (asset, balance) in balances {
					offchain_inventory
						.entry(asset)
						.and_modify(|total: &mut Decimal| {
							*total = (*total).saturating_add(balance);
						})
						.or_insert(balance);
				}
			}
			//      5. Load assets pallet balances of registered assets
			let assets = <AllowlistedToken<T>>::get();
			let mut onchain_inventory = BTreeMap::new();
			for asset in assets {
				let total = Self::get_onchain_balance(asset);
				onchain_inventory
					.entry(asset)
					.and_modify(|total_balance: &mut Decimal| {
						*total_balance = (*total_balance).saturating_add(total)
					})
					.or_insert(total);
			}
			//      6. Compute the sum of new balances on-chain using ingress messages
			let current_blk = frame_system::Pallet::<T>::current_block_number().saturated_into();
			if current_blk > last_processed_blk {
				for blk in last_processed_blk.saturating_add(1)..=current_blk {
					let ingress_msgs = <IngressMessages<T>>::get(blk);
					for msg in ingress_msgs {
						match msg {
							polkadex_primitives::ingress::IngressMessages::Deposit(
								_,
								asset,
								amt,
							) => {
								onchain_inventory
									.entry(asset)
									.and_modify(|total_balance| {
										*total_balance = (*total_balance).saturating_add(amt)
									})
									.or_insert(amt);
							},
							_ => {},
						}
					}
				}
			}
			//      7. Compute deviation and return it
			let mut deviation = BTreeMap::new();
			for asset in assets {
				let diff = onchain_inventory
					.get(&asset)
					.unwrap_or_default()
					.saturating_sub(*offchain_inventory.get(&asset).unwrap_or_default());
				deviation.insert(asset, diff);
			}
			return Ok(deviation)
		}
		Self::release_offchain_lock();
		Err(DispatchError::Other("Unable to calculate deviation"))
	}
}
