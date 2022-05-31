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
pub use polkadex_ido_runtime_api::PolkadexIdoRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use polkadex_primitives::assets::AssetId;
const RUNTIME_ERROR: i64 = 1;

#[rpc]
pub trait PolkadexIdoRpcApi<BlockHash, AccountId, Hash> {
    #[rpc(name = "polkadexido_getRoundsByInvestor")]
    fn get_rounds_by_investor(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Result<Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>>;
    #[rpc(name = "polkadexido_getRoundsByCreator")]
    fn get_rounds_by_creator(
        &self,
        account: AccountId,
        at: Option<BlockHash>,
    ) -> Result<Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>>;

    #[rpc(name = "polkadexido_getVotingStat")]
    fn get_voting_stats(
        &self,
        round_id: Hash,
        at: Option<BlockHash>,
    ) -> Result<VoteStat>;

    #[rpc(name = "polkadexido_activeRounds")]
    fn active_rounds(
        &self,
        at: Option<BlockHash>,
    ) -> Result<Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>>;
    #[rpc(name = "polkadexido_accountBalances")]
    fn account_balances(&self, assets : Vec<String>, account_id : AccountId,  at: Option<BlockHash>) ->  Result<Vec<String>>;
}

/// A struct that implements the `SumStorageApi`.
pub struct PolkadexIdoRpc<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> PolkadexIdoRpc<C, M> {
    /// Create new `SumStorage` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block, AccountId, Hash> PolkadexIdoRpcApi<<Block as BlockT>::Hash, AccountId, Hash>
    for PolkadexIdoRpc<C, Block>
where
    Block: BlockT,
    C: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
    C::Api: PolkadexIdoRuntimeApi<Block, AccountId, Hash>,
    AccountId: Codec,
    Hash: Codec,
{

    /// # RPC Call
    /// Returns rounds an investor has invested in
    /// ## Parameters
    /// * `account` : Account id
    fn get_rounds_by_investor(
        &self,
        account: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        api.rounds_by_investor(&at, account)
            .map_err(runtime_error_into_rpc_err)
    }

    /// # RPC Call
    /// Returns rounds an investor has invested in
    /// ## Parameters
    /// * `account` : Account id
    fn get_rounds_by_creator(
        &self,
        account: AccountId,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.rounds_by_creator(&at, account);
        runtime_api_result.map_err(runtime_error_into_rpc_err)
    }

    /// # RPC Call
    /// Returns Votes statistics for a round
    /// ## Parameters
    /// * `round_id` : Account id
    fn get_voting_stats(&self, round_id: Hash,at: Option<<Block as BlockT>::Hash>) -> Result<VoteStat> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.votes_stat(&at, round_id);
        runtime_api_result.map_err(runtime_error_into_rpc_err)
    }

    /// # RPC Call
    /// Returns rounds that are not closed
    fn active_rounds(
        &self,
        at: Option<<Block as BlockT>::Hash>,
    ) -> Result<Vec<(Hash, FundingRoundWithPrimitives<AccountId>)>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.active_rounds(&at);
        runtime_api_result.map_err(runtime_error_into_rpc_err)
    }

    /// # RPC Call
    /// Returns the asset balance(free balance) belonging to account_id
    /// ## Parameters
    /// * `assets` : List of assets
    /// * `account_id` : Account id
    fn account_balances(&self, assets : Vec<String>, account_id : AccountId, at: Option<<Block as BlockT>::Hash>) ->  Result<Vec<String>> {
        let assets : Result<Vec<_>> = assets.iter().map(|asset_id| {asset_id.parse::<u128>().map_err(runtime_error_into_rpc_err)}).collect();
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.account_balances(&at, assets?, account_id);
        runtime_api_result.map(|balances| {
            balances.iter().map(|balance| {
                balance.to_string()
            }).collect()
        }).map_err(runtime_error_into_rpc_err)
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
