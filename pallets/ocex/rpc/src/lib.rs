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

//! This crate provides an RPC methods for OCEX pallet - balances state and onchain/offchain
//! recovery data.

pub mod offchain;

use orderbook_primitives::{types::AccountAsset};
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	tracing::log,
	types::error::{CallError, ErrorObject},
};
use orderbook_primitives::recovery::{DeviationMap, ObCheckpoint, ObRecoveryState};
use pallet_ocex_lmp::{snapshot::StateInfo, validator::STATE_INFO};
pub use pallet_ocex_runtime_api::PolkadexOcexRuntimeApi;
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
use polkadex_primitives::{AccountId, AssetId};
use rust_decimal::Decimal;
use sc_rpc_api::DenyUnsafe;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_core::{offchain::OffchainStorage, ByteArray};
use sp_runtime::traits::Block as BlockT;
use std::{collections::BTreeMap, sync::Arc};

const RUNTIME_ERROR: i32 = 1;
const RETRIES: u8 = 3;

#[rpc(client, server)]
pub trait PolkadexOcexRpcApi<BlockHash, Hash> {
	#[method(name = "ob_getRecoverState")]
	async fn get_ob_recover_state(&self, at: Option<BlockHash>) -> RpcResult<ObRecoveryState>;

	#[method(name = "ob_getBalance")]
	async fn get_balance(
		&self,
		account_id: AccountId,
		of: AssetId,
	) -> RpcResult<String>;

	#[method(name = "ob_inventoryDeviation")]
	async fn calculate_inventory_deviation(&self, at: Option<BlockHash>) -> RpcResult<String>;

	#[method(name = "ob_fetchCheckpoint")]
	async fn fetch_checkpoint(&self, at: Option<BlockHash>) -> RpcResult<ObCheckpoint>;
}

/// A structure that represents the Polkadex OCEX pallet RPC, which allows querying
/// individual balances and recovery state data.
///
/// # Type Parameters
///
/// * `Client`: The client API used to interact with the Substrate runtime.
/// * `Block`: The block type of the Substrate.
pub struct PolkadexOcexRpc<Client, Block, T: OffchainStorage + 'static> {
	/// An `Arc` reference to the client API for accessing runtime functionality.
	client: Arc<Client>,

	/// Offchain storage
	storage: Arc<RwLock<T>>,
	deny_unsafe: DenyUnsafe,

	/// A marker for the `Block` type parameter, used to ensure the struct
	/// is covariant with respect to the block type.
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block, T: OffchainStorage> PolkadexOcexRpc<Client, Block, T> {
	pub fn new(client: Arc<Client>, storage: T, deny_unsafe: DenyUnsafe) -> Self {
		Self {
			client,
			storage: Arc::new(RwLock::new(storage)),
			deny_unsafe,
			_marker: Default::default(),
		}
	}

	pub fn get_offchain_balances(
		&self,
		state: &mut pallet_ocex_lmp::storage::OffchainState,
		account: &AccountId,
	) -> Result<BTreeMap<AssetId, Decimal>, &'static str> {
		match state.get(&account.to_raw_vec())? {
			None => Ok(BTreeMap::new()),
			Some(encoded) => BTreeMap::decode(&mut &encoded[..])
				.map_err(|_| "Unable to decode balances for account"),
		}
	}

	/// Loads the state info from the offchain state
	pub fn load_state_info(
		&self,
		state: &mut pallet_ocex_lmp::storage::OffchainState,
	) -> Result<StateInfo, &'static str> {
		match state.get(&STATE_INFO.to_vec())? {
			Some(data) => Ok(StateInfo::decode(&mut &data[..]).unwrap_or_default()),
			None => Ok(StateInfo::default()),
		}
	}
}

#[async_trait]
impl<Client, Block, Hash, T>
	PolkadexOcexRpcApiServer<<Block as BlockT>::Hash, Hash>
	for PolkadexOcexRpc<Client, Block, T>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: PolkadexOcexRuntimeApi<Block, AccountId, Hash>,
	Hash: Codec,
	T: OffchainStorage + 'static,
{
	async fn get_ob_recover_state(
		&self,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<ObRecoveryState> {

		// Acquire offchain storage lock
		let offchain_storage = offchain::OffchainStorageAdapter::new(self.storage.clone());
		if !offchain_storage.acquire_offchain_lock(3).await {
			return Err(runtime_error_into_rpc_err("Failed to acquire offchain lock"))
		}

		// 1. Load last processed blk
		let mut root = pallet_ocex_lmp::storage::load_trie_root();
		log::info!(target:"ocex-rpc","state_root {:?}", root);
		let mut storage = pallet_ocex_lmp::storage::State;
		let mut state = pallet_ocex_lmp::storage::OffchainState::load(&mut storage, &mut root);

		let api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		log::debug!(target:"ocex", "fetch_checkpoint called");
		let main_accounts: sp_std::collections::btree_map::BTreeMap<AccountId,sp_std::vec::Vec<AccountId>> = api.get_main_accounts(at).map_err(runtime_error_into_rpc_err)?;

		let mut balances: BTreeMap<AccountAsset, Decimal> = BTreeMap::new();
		// all offchain balances for main accounts
		for (main,_) in &main_accounts {
			let b = self.get_offchain_balances(&mut state, main.into()).map_err(runtime_error_into_rpc_err)?;
			for (asset, balance) in b.into_iter() {
				balances.insert(AccountAsset { main: main.clone(), asset }, balance);
			}
		}
		let state_info = self.load_state_info(&mut state).map_err(runtime_error_into_rpc_err)?;
		let last_processed_block_number = state_info.last_block;
		let snapshot_id = state_info.snapshot_id;
		let state_change_id = state_info.stid;
		log::debug!(target:"ocex", "fetch_checkpoint returning");
		Ok(ObRecoveryState{
			snapshot_id,
			account_ids:main_accounts,
			balances,
			last_processed_block_number,
			state_change_id,
			worker_nonce: 0
		})
	}

	async fn get_balance(
		&self,
		account_id: AccountId,
		of: AssetId,
	) -> RpcResult<String> {

		// Acquire offchain storage lock
		let offchain_storage = offchain::OffchainStorageAdapter::new(self.storage.clone());
		if !offchain_storage.acquire_offchain_lock(3).await {
			return Err(runtime_error_into_rpc_err("Failed to acquire offchain lock"))
		}

		let mut root = pallet_ocex_lmp::storage::load_trie_root();
		log::info!(target:"ocex-rpc","state_root {:?}", root);
		let mut storage = pallet_ocex_lmp::storage::State;
		let mut state = pallet_ocex_lmp::storage::OffchainState::load(&mut storage, &mut root);
		let balances = self.get_offchain_balances(&mut state,&account_id).map_err(runtime_error_into_rpc_err)?;
		let balance = balances.get(&of).copied().unwrap_or_default();
		let json = serde_json::to_string(&balance).map_err(runtime_error_into_rpc_err)?;
		Ok(json)
	}

	async fn calculate_inventory_deviation(
		&self,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<String> {
		self.deny_unsafe.check_if_safe()?;
		let api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};
		let offchain_storage = offchain::OffchainStorageAdapter::new(self.storage.clone());
		if !offchain_storage.acquire_offchain_lock(3).await {
			return Err(runtime_error_into_rpc_err("Failed to acquire offchain lock"))
		}

		log::info!(target:"ocex-rpc","calculating the inventory deviation..");
		// 1. Load last processed blk
		let mut root = pallet_ocex_lmp::storage::load_trie_root();
		log::info!(target:"ocex-rpc","state_root {:?}", root);
		let mut storage = pallet_ocex_lmp::storage::State;
		let mut state = pallet_ocex_lmp::storage::OffchainState::load(&mut storage, &mut root);
		let state_info = self.load_state_info(&mut state).map_err(runtime_error_into_rpc_err)?;
		let last_processed_blk = state_info.last_block;
		// 2. Load all main accounts and registered assets from on-chain
		let mut offchain_inventory = BTreeMap::new();
		let main_accounts = api.get_main_accounts(at).map_err(runtime_error_into_rpc_err)?;
		for main in main_accounts {
			// 3. Compute sum of all balances of all assets
			let balances: BTreeMap<AssetId, Decimal> = self
				.get_offchain_balances(&mut state, &Decode::decode(&mut &main.encode()[..]).unwrap())
				.map_err(runtime_error_into_rpc_err)?;
			for (asset, balance) in balances {
				offchain_inventory
					.entry(asset)
					.and_modify(|total: &mut Decimal| {
						*total = (*total).saturating_add(balance);
					})
					.or_insert(balance);
			}
		}

		let deviation = match api
			.calculate_inventory_deviation(at, offchain_inventory, last_processed_blk)
			.map_err(runtime_error_into_rpc_err)?
		{
			Err(err) => {
				log::error!(target:"ocex","Error calling calculate_inventory_deviation: {:?}",err);
				return Err(runtime_error_into_rpc_err(
					"Error calling calculate_inventory_deviation ",
				))
			},
			Ok(deviation_map) => DeviationMap::new(deviation_map),
		};
		log::info!(target:"ocex","serializing the deviation map..");
		let json = serde_json::to_string(&deviation).map_err(runtime_error_into_rpc_err)?;
		Ok(json)
	}

	async fn fetch_checkpoint(
		&self,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<ObCheckpoint> {
		//self.deny_unsafe.check_if_safe()?; //As it is used by the aggregator, we need to allow it
		let api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};
		let offchain_storage = offchain::OffchainStorageAdapter::new(self.storage.clone());
		if !offchain_storage.acquire_offchain_lock(RETRIES).await {
			return Err(runtime_error_into_rpc_err("Failed to acquire offchain lock"))
		}
		let ob_checkpoint_raw = api
			.fetch_checkpoint(at)
			.map_err(runtime_error_into_rpc_err)?
			.map_err(runtime_error_into_rpc_err)?;
		let ob_checkpoint = ob_checkpoint_raw.to_checkpoint();
		Ok(ob_checkpoint)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	log::error!(target:"ocex","runtime rpc error: {:?} ",err);
	CallError::Custom(ErrorObject::owned(RUNTIME_ERROR, "Runtime error", Some(format!("{err:?}"))))
		.into()
}
