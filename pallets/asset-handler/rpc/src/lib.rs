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

use std::sync::Arc;

use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
pub use pallet_asset_handler_runtime_api::PolkadexAssetHandlerRuntimeApi;
use parity_scale_codec::Codec;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

const RUNTIME_ERROR: i32 = 1;

#[rpc(client, server)]
pub trait PolkadexAssetHandlerRpcApi<BlockHash, AccountId, Hash> {
	#[method(name = "assethandler_accountbalances")]
	fn account_balances(
		&self,
		assets: Vec<String>,
		account_id: AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<String>>;
}

pub struct PolkadexAssetHandlerRpc<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> PolkadexAssetHandlerRpc<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<Client, Block, AccountId, Hash>
	PolkadexAssetHandlerRpcApiServer<<Block as BlockT>::Hash, AccountId, Hash>
	for PolkadexAssetHandlerRpc<Client, Block>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: PolkadexAssetHandlerRuntimeApi<Block, AccountId, Hash>,
	AccountId: Codec,
	Hash: Codec,
{
	fn account_balances(
		&self,
		assets: Vec<String>,
		account_id: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<String>> {
		let assets: RpcResult<Vec<_>> = assets
			.iter()
			.map(|asset_id| asset_id.parse::<u128>().map_err(runtime_error_into_rpc_err))
			.collect();
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
		// If the block hash is not supplied assume the best block.
		self.client.info().best_hash));

		let runtime_api_result = api.account_balances(&at, assets?, account_id);
		runtime_api_result
			.map(|balances| balances.iter().map(|balance| balance.to_string()).collect())
			.map_err(runtime_error_into_rpc_err)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(RUNTIME_ERROR, "Runtime error", Some(format!("{err:?}"))))
		.into()
}
