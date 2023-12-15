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

use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	tracing::log,
	types::error::{CallError, ErrorObject},
};
pub use pallet_asset_conversion::AssetConversionApi;
use polkadex_primitives::AssetId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block as BlockT;
use std::sync::Arc;

const RUNTIME_ERROR: i32 = 1;

#[rpc(client, server)]
pub trait PolkadexSwapRpcApi<BlockHash> {
	#[method(name = "tx_quotePriceExactTokensForTokens")]
	async fn quote_price_exact_tokens_for_tokens(
		&self,
		asset_id1: String,
		asset_id2: String,
		amount: u128,
		include_fee: bool,
	) -> RpcResult<Option<u128>>;

	#[method(name = "tx_quotePriceTokensForExactTokens")]
	async fn quote_price_tokens_for_exact_tokens(
		&self,
		asset_id1: String,
		asset_id2: String,
		amount: u128,
		include_fee: bool,
	) -> RpcResult<Option<u128>>;
}

/// A structure that represents the Polkadex OCEX pallet RPC, which allows querying
/// individual balances and recovery state data.
///
/// # Type Parameters
///
/// * `Client`: The client API used to interact with the Substrate runtime.
/// * `Block`: The block type of the Substrate.
pub struct PolkadexSwapRpc<Client, Block> {
	/// An `Arc` reference to the client API for accessing runtime functionality.
	client: Arc<Client>,
	/// A marker for the `Block` type parameter, used to ensure the struct
	/// is covariant with respect to the block type.
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> PolkadexSwapRpc<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<Client, Block> PolkadexSwapRpcApiServer<<Block as BlockT>::Hash>
	for PolkadexSwapRpc<Client, Block>
where
	Block: BlockT,
	Client: ProvideRuntimeApi<Block> + Send + Sync + 'static + HeaderBackend<Block>,
	Client::Api: pallet_asset_conversion::AssetConversionApi<Block, u128, u128, AssetId>,
{
	async fn quote_price_exact_tokens_for_tokens(
		&self,
		asset_id1: String,
		asset_id2: String,
		amount: u128,
		include_fee: bool,
	) -> RpcResult<Option<u128>> {
		let api = self.client.runtime_api();
		let at = self.client.info().best_hash;
		let asset_id1: AssetId = AssetId::try_from(asset_id1).map_err(runtime_error_into_rpc_err)?;
		let asset_id2: AssetId = AssetId::try_from(asset_id2).map_err(runtime_error_into_rpc_err)?;
		let runtime_api_result = api
			.quote_price_exact_tokens_for_tokens(at, asset_id1, asset_id2, amount, include_fee)
			.map_err(runtime_error_into_rpc_err)?;
		Ok(runtime_api_result)
	}

	async fn quote_price_tokens_for_exact_tokens(
		&self,
		asset_id1: String,
		asset_id2: String,
		amount: u128,
		include_fee: bool,
	) -> RpcResult<Option<u128>> {
		let api = self.client.runtime_api();
		let at = self.client.info().best_hash;
		let asset_id1: AssetId = AssetId::try_from(asset_id1).map_err(runtime_error_into_rpc_err)?;
		let asset_id2: AssetId = AssetId::try_from(asset_id2).map_err(runtime_error_into_rpc_err)?;
		let runtime_api_result = api
			.quote_price_tokens_for_exact_tokens(at, asset_id1, asset_id2, amount, include_fee)
			.map_err(runtime_error_into_rpc_err)?;
		Ok(runtime_api_result)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	log::error!(target:"ocex","runtime rpc error: {:?} ",err);
	CallError::Custom(ErrorObject::owned(RUNTIME_ERROR, "Runtime error", Some(format!("{err:?}"))))
		.into()
}
