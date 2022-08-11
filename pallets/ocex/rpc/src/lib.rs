// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use codec::Codec;
use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use pallet_polkadex_ido_primitives::{FundingRoundWithPrimitives, VoteStat};
pub use pollet_ocex_lmp_runtime_api::PolkadexOcexRuntimeApi;
use polkadex_primitives::assets::AssetId;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
const RUNTIME_ERROR: i64 = 1;

#[rpc]
pub trait PolkadexOcexRpcApi<BlockHash, AccountId, Hash, Balance> {
	#[rpc(name = "pallet_ocex_return_withdrawals")]
    fn return_withdrawals(snapshot_ids: Vec<u32>,account: AccountId) -> Result<Vec<Withdrawal<AccountId, Balance>>>;
}

/// A struct that implements the `SumStorageApi`.
pub struct PolkadexIdoRpc<C, M> {
	client: Arc<C>,
	_marker: std::marker::PhantomData<M>,
}

impl<C, M> PolkadexIdoRpc<C, M> {
	/// Create new `SumStorage` instance with the given reference to the client.
	pub fn new(client: Arc<C>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

impl<C, Block, AccountId, Hash> PolkadexOcexRpcApi<<Block as BlockT>::Hash, AccountId, Hash, Balance>
	for PolkadexIdoRpc<C, Block>
where
	Block: BlockT,
	C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	C::Api: PolkadexOcexRuntimeApi<Block, AccountId, Hash>,
	AccountId: Codec,
	Hash: Codec,
    Balance: Zero + Clone
{
	/// # RPC Call
	/// Returns rounds an investor has invested in
	/// ## Parameters
	/// * `account` : Account id
	fn return_withdrawals(
		&self,
        snapshot_ids: Vec<u32>,
		account: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> Result<Vec<Withdrawal<AccountId, Balance>>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

		api.return_withdrawals(&at, snapshot_ids, account).map_err(runtime_error_into_rpc_err)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> RpcError {
	RpcError {
		code: ErrorCode::ServerError(RUNTIME_ERROR),
		message: "Runtime error".into(),
		data: Some(format!("{:?}", err).into()),
	}
}
