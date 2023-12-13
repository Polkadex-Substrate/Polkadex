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
use parity_scale_codec::{Codec, Decode};
use polkadex_primitives::AssetId;
use sc_rpc_api::DenyUnsafe;
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_core::offchain::{storage::OffchainDb, OffchainDbExt, OffchainStorage};
use sp_runtime::traits::Block as BlockT;
use pallet_asset_conversion::{NativeOrAssetId, NativeOrAssetIdConverter};
use std::sync::Arc;
pub use pallet_asset_conversion::AssetConversionApi;

const RUNTIME_ERROR: i32 = 1;
const RETRIES: u8 = 3;

#[rpc(client, server)]
pub trait PolkadexSwapRpcApi<BlockHash, AccountId, Hash> {
	#[method(name = "tx_quotePriceExactTokensForTokens")]
	async fn quote_price_exact_tokens_for_tokens(&self, at: Option<BlockHash>, is_native_asset1: bool, asset_id1: u128, is_native_asset2: bool, asset_id2: u128, amount: u128, include_fee: bool) -> RpcResult<Option<u128>>;

	#[method(name = "tx_quotePriceTokensForExactTokens")]
	async fn quote_price_tokens_for_exact_tokens(&self, at: Option<BlockHash>, is_native_asset1: bool, asset_id1: u128, is_native_asset2: bool, asset_id2: u128, amount: u128, include_fee: bool) -> RpcResult<Option<u128>>;
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

	deny_unsafe: DenyUnsafe,

	/// A marker for the `Block` type parameter, used to ensure the struct
	/// is covariant with respect to the block type.
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> PolkadexSwapRpc<Client, Block> {
	pub fn new(client: Arc<Client>, deny_unsafe: DenyUnsafe) -> Self {
		Self {
			client,
			deny_unsafe,
			_marker: Default::default(),
		}
	}
}

#[async_trait]
impl<Client, Block, AccountId, Hash>
    PolkadexSwapRpcApiServer<<Block as BlockT>::Hash, AccountId, Hash>
	for PolkadexSwapRpc<Client, Block>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: pallet_asset_conversion::AssetConversionApi<
		Block,
		u128,
		u128,
		NativeOrAssetId<u128>>,
	AccountId: Codec,
	Hash: Codec,
{

	async fn quote_price_exact_tokens_for_tokens(&self, at: Option<<Block as BlockT>::Hash>, is_native_asset1: bool, asset_id1: u128, is_native_asset2: bool, asset_id2: u128, amount: u128, include_fee: bool) -> RpcResult<Option<u128>> {
		let mut api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};
		let asset1: NativeOrAssetId<u128> = if is_native_asset1 {
			NativeOrAssetId::Native
		} else {
			NativeOrAssetId::Asset(asset_id1)
		};
		let asset2: NativeOrAssetId<u128> = if is_native_asset2 {
			NativeOrAssetId::Native
		} else {
			NativeOrAssetId::Asset(asset_id2)
		};
		let runtime_api_result = api
			.quote_price_exact_tokens_for_tokens(at, asset1, asset2, amount, include_fee)
			.map_err(runtime_error_into_rpc_err)?;
		Ok(runtime_api_result)
	}

	async fn quote_price_tokens_for_exact_tokens(&self, at: Option<<Block as BlockT>::Hash>, is_native_asset1: bool, asset_id1: u128, is_native_asset2: bool, asset_id2: u128, amount: u128, include_fee: bool) -> RpcResult<Option<u128>> {
		let mut api = self.client.runtime_api();
		let at = match at {
			Some(at) => at,
			None => self.client.info().best_hash,
		};
		let asset1: NativeOrAssetId<u128> = if is_native_asset1 {
			NativeOrAssetId::Native
		} else {
			NativeOrAssetId::Asset(asset_id1)
		};
		let asset2: NativeOrAssetId<u128> = if is_native_asset2 {
			NativeOrAssetId::Native
		} else {
			NativeOrAssetId::Asset(asset_id2)
		};
		let runtime_api_result = api
			.quote_price_tokens_for_exact_tokens(at, asset1, asset2, amount, include_fee)
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
