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

//! This crate provides an RPC method "accountInfo" to retrieve rewards related information.

use std::sync::Arc;

use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
pub use pallet_rewards_runtime_api::PolkadexRewardsRuntimeApi;
use parity_scale_codec::Codec;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};

const RUNTIME_ERROR: i32 = 1;

#[rpc(client, server)]
pub trait PolkadexRewardsRpcApi<BlockHash, AccountId, Hash> {
	#[method(name = "rewards_accountInfo")]
	fn account_info(
		&self,
		account_id: AccountId,
		reward_id: u32,
		at: Option<BlockHash>,
	) -> RpcResult<String>;
}

/// A structure that represents the Polkadex Rewards RPC, which allows querying
/// rewards-related information through remote procedure calls.
///
/// # Type Parameters
///
/// * `Client`: The client API used to interact with the Substrate runtime.
/// * `Block`: The block type of the Substrate runtime.
pub struct PolkadexRewardsRpc<Client, Block> {
	/// An `Arc` reference to the client API for accessing runtime functionality.
	client: Arc<Client>,

	/// A marker for the `Block` type parameter, used to ensure the struct
	/// is covariant with respect to the block type.
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> PolkadexRewardsRpc<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<Client, Block, AccountId, Hash>
	PolkadexRewardsRpcApiServer<<Block as BlockT>::Hash, AccountId, Hash>
	for PolkadexRewardsRpc<Client, Block>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: PolkadexRewardsRuntimeApi<Block, AccountId, Hash>,
	AccountId: Codec,
	Hash: Codec,
{
	fn account_info(
		&self,
		account_id: AccountId,
		reward_id: u32,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<String> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		let runtime_api_result = api
			.account_info(&at, account_id, reward_id)
			.map_err(runtime_error_into_rpc_err)?;
		let json =
			serde_json::to_string(&runtime_api_result).map_err(runtime_error_into_rpc_err)?;
		Ok(json)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(RUNTIME_ERROR, "Runtime error", Some(format!("{err:?}"))))
		.into()
}
