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

use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	tracing::log,
	types::error::{CallError, ErrorObject},
};
use orderbook_primitives::{
	recovery::{DeviationMap, ObCheckpoint, ObRecoveryState},
	types::TradingPair,
};
pub use pallet_ocex_runtime_api::PolkadexOcexRuntimeApi;
use parity_scale_codec::{Codec, Decode};
use polkadex_primitives::AssetId;
use rust_decimal::Decimal;
use sc_rpc_api::DenyUnsafe;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::offchain::{storage::OffchainDb, OffchainDbExt, OffchainStorage};
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

const RUNTIME_ERROR: i32 = 1;
const RETRIES: u8 = 3;

#[rpc(client, server)]
pub trait PolkadexOcexRpcApi<BlockHash, AccountId, Hash> {
	#[method(name = "ob_getRecoverState")]
	fn get_ob_recover_state(&self, at: Option<BlockHash>) -> RpcResult<ObRecoveryState>;

	#[method(name = "ob_getBalance")]
	fn get_balance(
		&self,
		account_id: AccountId,
		of: AssetId,
		at: Option<BlockHash>,
	) -> RpcResult<String>;

	#[method(name = "ob_inventoryDeviation")]
	async fn calculate_inventory_deviation(&self, at: Option<BlockHash>) -> RpcResult<String>;

	#[method(name = "ob_fetchCheckpoint")]
	async fn fetch_checkpoint(&self, at: Option<BlockHash>) -> RpcResult<ObCheckpoint>;

	#[method(name = "lmp_accountsSorted")]
	async fn account_scores_by_market(
		&self,
		epoch: u16,
		market: String,
		sorted_by_mm_score: bool,
		limit: u16,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<AccountId>>;

	#[method(name = "lmp_eligibleRewards")]
	fn eligible_rewards(
		&self,
		epoch: u16,
		market: String,
		main: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<(String, String, bool)>;

	#[method(name = "lmp_feesPaidByUserPerEpoch")]
	fn get_fees_paid_by_user_per_epoch(
		&self,
		epoch: u16,
		market: String,
		main: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<String>;

	#[method(name = "lmp_volumeGeneratedByUserPerEpoch")]
	fn get_volume_by_user_per_epoch(
		&self,
		epoch: u16,
		market: String,
		main: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<String>;

	#[method(name = "lmp_listClaimableEpochs")]
	fn list_claimable_epochs(
		&self,
		market: String,
		main: AccountId,
		until_epoch: u16,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<u16>>;
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
	offchain_db: OffchainDb<T>,
	deny_unsafe: DenyUnsafe,

	/// A marker for the `Block` type parameter, used to ensure the struct
	/// is covariant with respect to the block type.
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block, T: OffchainStorage> PolkadexOcexRpc<Client, Block, T> {
	pub fn new(client: Arc<Client>, storage: T, deny_unsafe: DenyUnsafe) -> Self {
		Self {
			client,
			offchain_db: OffchainDb::new(storage),
			deny_unsafe,
			_marker: Default::default(),
		}
	}
}

#[async_trait]
impl<Client, Block, AccountId, Hash, T>
	PolkadexOcexRpcApiServer<<Block as BlockT>::Hash, AccountId, Hash>
	for PolkadexOcexRpc<Client, Block, T>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: PolkadexOcexRuntimeApi<Block, AccountId, Hash>,
	AccountId: Codec + Clone,
	Hash: Codec,
	T: OffchainStorage + 'static,
{
	fn get_ob_recover_state(
		&self,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<ObRecoveryState> {
		let mut api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		// WARN: this is a hack on beating the boundry of runtime ->
		// polkadex-node with decoding tuple of underlying data into
		// solid std type
		Decode::decode(
			&mut api
				.get_ob_recover_state(at)
				.map_err(runtime_error_into_rpc_err)?
				.map_err(runtime_error_into_rpc_err)?
				.as_ref(),
		)
		.map_err(runtime_error_into_rpc_err)
	}

	fn get_balance(
		&self,
		account_id: AccountId,
		of: AssetId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<String> {
		let mut api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let runtime_api_result =
			api.get_balance(at, account_id, of).map_err(runtime_error_into_rpc_err)?;
		let json =
			serde_json::to_string(&runtime_api_result).map_err(runtime_error_into_rpc_err)?;
		Ok(json)
	}

	async fn calculate_inventory_deviation(
		&self,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<String> {
		self.deny_unsafe.check_if_safe()?;
		let mut api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let mut offchain_storage = offchain::OffchainStorageAdapter::new(self.offchain_db.clone());
		if !offchain_storage.acquire_offchain_lock(3).await {
			return Err(runtime_error_into_rpc_err("Failed to acquire offchain lock"));
		}
		log::info!(target:"ocex","calculating the inventory deviation..");
		let deviation =
			match api.calculate_inventory_deviation(at).map_err(runtime_error_into_rpc_err)? {
				Err(err) => {
					log::error!(target:"ocex","Error calling calculate_inventory_deviation: {:?}",err);
					return Err(runtime_error_into_rpc_err(
						"Error calling calculate_inventory_deviation ",
					));
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
		let mut api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let mut offchain_storage = offchain::OffchainStorageAdapter::new(self.offchain_db.clone());
		if !offchain_storage.acquire_offchain_lock(RETRIES).await {
			return Err(runtime_error_into_rpc_err("Failed to acquire offchain lock"));
		}
		let ob_checkpoint_raw = api
			.fetch_checkpoint(at)
			.map_err(runtime_error_into_rpc_err)?
			.map_err(runtime_error_into_rpc_err)?;
		let ob_checkpoint = ob_checkpoint_raw.to_checkpoint();
		Ok(ob_checkpoint)
	}

	async fn account_scores_by_market(
		&self,
		epoch: u16,
		market: String,
		sorted_by_mm_score: bool,
		limit: u16,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<AccountId>> {
		let mut api = self.client.runtime_api();
		let market = TradingPair::try_from(market).map_err(runtime_error_into_rpc_err)?;
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		let accounts: Vec<AccountId> = api
			.top_lmp_accounts(at, epoch, market, sorted_by_mm_score, limit)
			.map_err(runtime_error_into_rpc_err)?;

		Ok(accounts)
	}

	fn eligible_rewards(
		&self,
		epoch: u16,
		market: String,
		main: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<(String, String, bool)> {
		let mut api = self.client.runtime_api();
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let market = TradingPair::try_from(market).map_err(runtime_error_into_rpc_err)?;
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		let (mm_rewards, trading_rewards, is_claimed) = api
			.calculate_lmp_rewards(at, main, epoch, market)
			.map_err(runtime_error_into_rpc_err)?;

		Ok((mm_rewards.to_string(), trading_rewards.to_string(), is_claimed))
	}

	fn get_fees_paid_by_user_per_epoch(
		&self,
		epoch: u16,
		market: String,
		main: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<String> {
		let mut api = self.client.runtime_api();
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let market = TradingPair::try_from(market).map_err(runtime_error_into_rpc_err)?;
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		let fees_paid: Decimal = api
			.get_fees_paid_by_user_per_epoch(at, epoch.into(), market, main)
			.map_err(runtime_error_into_rpc_err)?;

		Ok(fees_paid.to_string())
	}

	fn get_volume_by_user_per_epoch(
		&self,
		epoch: u16,
		market: String,
		main: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<String> {
		let mut api = self.client.runtime_api();
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let market = TradingPair::try_from(market).map_err(runtime_error_into_rpc_err)?;
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		let volume_generated: Decimal = api
			.get_volume_by_user_per_epoch(at, epoch.into(), market, main)
			.map_err(runtime_error_into_rpc_err)?;

		Ok(volume_generated.to_string())
	}

	fn list_claimable_epochs(
		&self,
		market: String,
		main: AccountId,
		until_epoch: u16,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<u16>> {
		let mut api = self.client.runtime_api();
		api.register_extension(OffchainDbExt::new(self.offchain_db.clone()));
		let market = TradingPair::try_from(market).map_err(runtime_error_into_rpc_err)?;
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};

		let mut claimable_epochs = Vec::new();

		for epoch in 0..=until_epoch {
			let (mm_rewards, trading_rewards, is_claimed) = api
				.calculate_lmp_rewards(at, main.clone(), epoch, market)
				.map_err(runtime_error_into_rpc_err)?;
			// If any one of the rewards are present and is_claimed is false,
			// then its claimable
			if (!mm_rewards.is_zero() || !trading_rewards.is_zero()) && !is_claimed {
				claimable_epochs.push(epoch)
			}
		}

		Ok(claimable_epochs)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	log::error!(target:"ocex","runtime rpc error: {:?} ",err);
	CallError::Custom(ErrorObject::owned(RUNTIME_ERROR, "Runtime error", Some(format!("{err:?}"))))
		.into()
}
