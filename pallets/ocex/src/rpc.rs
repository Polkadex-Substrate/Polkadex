// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

use crate::{
	pallet::{Accounts, AllowlistedToken, IngressMessages},
	storage::OffchainState,
	validator::WORKER_STATUS,
	Config, Pallet,
};
use frame_system::pallet_prelude::BlockNumberFor;
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{AccountId, AssetId};
use rust_decimal::Decimal;
use sp_application_crypto::ByteArray;
use sp_runtime::{
	offchain::storage::{StorageRetrievalError, StorageValueRef},
	traits::BlockNumberProvider,
	DispatchError, SaturatedConversion,
};
use sp_std::collections::btree_map::BTreeMap;

impl<T: Config> Pallet<T> {
	/// Try to acquire the offchain storage lock ( tries for 3 times )
	/// Return OK(()) if lock is acquired else , Err(())
	pub fn acquire_offchain_lock() -> Result<(), DispatchError> {
		// Check if another worker is already running or not
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		for _ in 0..3 {
			if s_info
				.mutate(|value: Result<Option<bool>, StorageRetrievalError>| -> Result<bool, ()> {
					match value {
						Ok(Some(true)) => {
							log::warn!(target:"ocex","Another worker is online, retrying after 1 sec");
							Err(())
						},
						Ok(Some(false)) | Ok(None) => Ok(true),
						Err(x) => {
							log::error!(target:"ocex","Error while acquiring lock: {:?}",x);
							Err(())
						},
					}
				})
				.is_ok()
			{
				return Ok(())
			}
		}
		Err(DispatchError::Other("Offchain storage mutex error"))
	}

	/// Release offchain storage lock
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
		// 1. Load last processed blk
		let mut root = crate::storage::load_trie_root();
		log::info!(target:"ocex-rpc","state_root {:?}", root);
		let mut storage = crate::storage::State;
		let mut state = OffchainState::load(&mut storage, &mut root);
		let state_info = Self::load_state_info(&mut state)?;
		let last_processed_blk = state_info.last_block;
		//      2. Load all main accounts and registered assets from on-chain
		let mut offchain_inventory = BTreeMap::new();
		for (main, _) in <Accounts<T>>::iter() {
			//      3. Compute sum of all balances of all assets
			let balances: BTreeMap<AssetId, Decimal> =
				Self::get_balances(&mut state, &Decode::decode(&mut &main.encode()[..]).unwrap())?;
			for (asset, balance) in balances {
				offchain_inventory
					.entry(asset)
					.and_modify(|total: &mut Decimal| {
						*total = (*total).saturating_add(balance);
					})
					.or_insert(balance);
			}
		}
		// 4. Load assets pallet balances of registered assets
		let assets = <AllowlistedToken<T>>::get();
		let mut onchain_inventory = BTreeMap::new();
		for asset in &assets {
			// There is no race condition here, as it will be computed for a given block
			let total = Self::get_onchain_balance(*asset);
			onchain_inventory
				.entry(*asset)
				.and_modify(|total_balance: &mut Decimal| {
					*total_balance = (*total_balance).saturating_add(total)
				})
				.or_insert(total);
		}
		// 5. Compute the sum of new balances on-chain using ingress messages
		let current_blk = frame_system::Pallet::<T>::current_block_number().saturated_into();
		if current_blk > last_processed_blk {
			for blk in last_processed_blk.saturating_add(1)..=current_blk {
				let ingress_msgs =
					<IngressMessages<T>>::get(blk.saturated_into::<BlockNumberFor<T>>());
				for msg in ingress_msgs {
					if let polkadex_primitives::ingress::IngressMessages::Deposit(_, asset, amt) =
						msg
					{
						onchain_inventory
							.entry(asset)
							.and_modify(|total_balance| {
								*total_balance = (*total_balance).saturating_add(amt)
							})
							.or_insert(amt);
					}
				}
			}
		}
		// 6. Compute deviation and return it
		let mut deviation = BTreeMap::new();
		for asset in &assets {
			let diff = onchain_inventory
				.get(asset)
				.cloned()
				.unwrap_or_default()
				.saturating_sub(offchain_inventory.get(asset).cloned().unwrap_or_default());
			deviation.insert(*asset, diff);
		}
		Ok(deviation)
	}
}
