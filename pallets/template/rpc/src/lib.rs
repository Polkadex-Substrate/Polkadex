//! RPC interface for the transaction payment module.

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use std::sync::Arc;
use runtime_api::DexStorageApi as DexStorageRuntimeApi;
use sp_arithmetic::FixedU128;
use sp_core::H256;
use sp_std::vec::Vec;
use pallet_template::LinkedPriceLevel;
use pallet_template::Trait;

#[rpc]
pub trait DexStorageApi<BlockHash,K> where K:Trait {
    #[rpc(name = "get_ask_level")]
    fn get_ask_level(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<Vec<FixedU128>>;

    #[rpc(name = "get_bid_level")]
    fn get_bid_level(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<Vec<FixedU128>>;

    #[rpc(name = "get_price_level")]
    fn get_price_level(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<Vec<LinkedPriceLevel<K>>>;
}

/// A struct that implements the `SumStorageApi`.
pub struct DexStorage<C, M> {
    // If you have more generics, no need to SumStorage<C, M, N, P, ...>
    // just use a tuple like SumStorage<C, (M, N, P, ...)>
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> DexStorage<C, M> {
    /// Create new `SumStorage` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

/// Error type of this RPC api.
// pub enum Error {
// 	/// The transaction was not decodable.
// 	DecodeError,
// 	/// The call to runtime failed.
// 	RuntimeError,
// }
//
// impl From<Error> for i64 {
// 	fn from(e: Error) -> i64 {
// 		match e {
// 			Error::RuntimeError => 1,
// 			Error::DecodeError => 2,
// 		}
// 	}
// }

impl<C, Block, K> DexStorageApi<<Block as BlockT>::Hash, K > for DexStorage<C, Block>
    where
        Block: BlockT,
        C: Send + Sync + 'static,
        C: ProvideRuntimeApi<Block>,
        C: HeaderBackend<Block>,
        K: Trait,
        C::Api: DexStorageRuntimeApi<Block, K>,
{
    fn get_ask_level(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<Vec<FixedU128>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            // Always take the best block hash for this RPC
            self.client.info().best_hash);

        // let hash_trading_pair = H256::from(trading_pair);
        let runtime_api_result = api.get_ask_level(&at, trading_pair);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(9876), // No real reason for this value
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn get_bid_level(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<Vec<FixedU128>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            // Always take the best block hash for this RPC
            self.client.info().best_hash);

        // let hash_trading_pair = H256::from(trading_pair);
        let runtime_api_result = api.get_bid_level(&at, trading_pair);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(9876), // No real reason for this value
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

    fn get_price_level(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<Vec<LinkedPriceLevel<K>>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            // Always take the best block hash for this RPC
            self.client.info().best_hash);

        // let hash_trading_pair = H256::from(trading_pair);
        let runtime_api_result = api.get_price_level(&at, trading_pair);
        runtime_api_result.map_err(|e| RpcError {
            code: ErrorCode::ServerError(9876), // No real reason for this value
            message: "Something wrong".into(),
            data: Some(format!("{:?}", e).into()),
        })
    }

}
