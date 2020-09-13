mod orderbook_type;

use std::sync::Arc;

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::generic::BlockId;
use sp_runtime::traits::Block as BlockT;

use pallet_dex::DexRuntimeApi;
use crate::orderbook_type::{convert_order_book_api_to_outer_order_book_api, OuterOrderBookApi};


#[rpc]
pub trait DexApi<BlockHash> {
    #[rpc(name = "dex_getOrderbook")]
    fn get_orderbook(&self, at: Option<BlockHash>, trading_pair: u32) ->Result<OuterOrderBookApi>;
}

/// A struct that implements the `DEX`.
pub struct DEX<C, M> {
    // If you have more generics, no need to DEX<C, M, N, P, ...>
    // just use a tuple like DEX<C, (M, N, P, ...)>
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> DEX<C, M> {
    /// Create new `DEX` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

impl<C, Block> DexApi<<Block as BlockT>::Hash> for DEX<C, Block>
    where
        Block: sp_runtime::traits::Block,
        C: Send + Sync + 'static,
        C: ProvideRuntimeApi<Block>,
        C: HeaderBackend<Block>,
        C::Api: DexRuntimeApi<Block>,
{
    fn get_orderbook(&self, at: Option<<Block as BlockT>::Hash>, trading_pair: u32) -> Result<OuterOrderBookApi> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

        let runtime_api_result = api.get_order_book(&at, trading_pair);
        if runtime_api_result.is_ok(){
            let orderbook = convert_order_book_api_to_outer_order_book_api(runtime_api_result.unwrap());
            Ok(orderbook)
        }else{
            Err("Something is wrong with RPC").map_err(|e| RpcError {
                code: ErrorCode::ServerError(1234), // No real reason for this value
                message: "Something wrong at the RPC side of node".into(),
                data: Some(format!("{:?}", e).into()),
            })
        }
    }
}