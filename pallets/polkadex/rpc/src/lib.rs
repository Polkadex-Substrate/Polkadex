//! RPC interface for the transaction payment module.

use std::sync::Arc;

use jsonrpc_core::{Error as RpcError, ErrorCode, Result};
use jsonrpc_derive::rpc;
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::FixedU128;
use sp_blockchain::HeaderBackend;
use sp_core::H256;
use sp_runtime::{generic::BlockId, traits::Block as BlockT};
use sp_std::vec::Vec;

use pallet_polkadex::data_structure_rpc::{ErrorRpc, LinkedPriceLevelRpc, MarketDataRpc, OrderbookRpc, OrderbookUpdates};
use runtime_api::DexStorageApi as DexStorageRuntimeApi;

#[rpc]
pub trait DexStorageApi<BlockHash> {
    #[rpc(name = "polkadex_getAskLevel")]
    fn get_ask_level(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<Vec<FixedU128>>;

    #[rpc(name = "polkadex_getBidLevel")]
    fn get_bid_level(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<Vec<FixedU128>>;

    #[rpc(name = "polkadex_getPriceLevel")]
    fn get_price_level(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<Vec<LinkedPriceLevelRpc>>;

    #[rpc(name = "polkadex_getOrderbook")]
    fn get_orderbook(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<OrderbookRpc>;

    #[rpc(name = "polkadex_getAllOrderbook")]
    fn get_all_orderbook(&self, at: Option<BlockHash>) -> Result<Vec<OrderbookRpc>>;

    #[rpc(name = "polkadex_getMarketInfo")]
    fn get_market_info(&self, at: Option<BlockHash>, trading_pair: H256, blocknum: u32) -> Result<MarketDataRpc>;

    #[rpc(name = "polkadex_getOrderbookUpdates")]
    fn get_orderbook_updates(&self, at: Option<BlockHash>, trading_pair: H256) -> Result<OrderbookUpdates>;
}

/// A struct that implements the `DexStorageApi`.
pub struct DexStorage<C, M> {
    client: Arc<C>,
    _marker: std::marker::PhantomData<M>,
}

impl<C, M> DexStorage<C, M> {
    /// Create new `DexStorage` instance with the given reference to the client.
    pub fn new(client: Arc<C>) -> Self {
        Self {
            client,
            _marker: Default::default(),
        }
    }
}

/// Error type of this RPC api.
pub struct ErrorConvert;

impl ErrorConvert {
    fn covert_to_rpc_error(error_type: ErrorRpc) -> RpcError {
        match error_type {
            ErrorRpc::IdMustBe32Byte => RpcError {
                code: ErrorCode::ServerError(1000),
                message: "IdMustBe32Byte".into(),
                data: Some(format!("{:?}", error_type).into()),
            },
            ErrorRpc::AssetIdConversionFailed => RpcError {
                code: ErrorCode::ServerError(1001),
                message: "AssetIdConversionFailed".into(),
                data: Some(format!("{:?}", error_type).into()),
            },
            ErrorRpc::Fixedu128tou128conversionFailed => RpcError {
                code: ErrorCode::ServerError(1002),
                message: "Fixedu128tou128conversionFailed".into(),
                data: Some(format!("{:?}", error_type).into()),
            },
            ErrorRpc::NoElementFound => RpcError {
                code: ErrorCode::ServerError(1003),
                message: "NoElementFound".into(),
                data: Some(format!("{:?}", error_type).into()),
            },
            ErrorRpc::ServerErrorWhileCallingAPI => RpcError {
                code: ErrorCode::ServerError(1004),
                message: "ServerErrorWhileCallingAPI".into(),
                data: Some(format!("{:?}", error_type).into()),
            },
        }
    }
}


impl<C, Block> DexStorageApi<<Block as BlockT>::Hash> for DexStorage<C, Block>
    where
        Block: BlockT,
        C: Send + Sync + 'static,
        C: ProvideRuntimeApi<Block>,
        C: HeaderBackend<Block>,
        C::Api: DexStorageRuntimeApi<Block>,
{
    fn get_ask_level(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<Vec<FixedU128>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            self.client.info().best_hash);
        let runtime_api_result = api.get_ask_level(&at, trading_pair);
        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI), // change
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }

    fn get_bid_level(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<Vec<FixedU128>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            self.client.info().best_hash);

        let runtime_api_result = api.get_bid_level(&at, trading_pair);

        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI), // change
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }

    fn get_price_level(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<Vec<LinkedPriceLevelRpc>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(

            self.client.info().best_hash);


        let runtime_api_result = api.get_price_level(&at, trading_pair);
        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI), // change
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }

    fn get_orderbook(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<OrderbookRpc> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            // Always take the best block hash for this RPC
            self.client.info().best_hash);

        // let hash_trading_pair = H256::from(trading_pair);
        let runtime_api_result = api.get_orderbook(&at, trading_pair);
        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI),
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }

    fn get_all_orderbook(&self, _at: Option<<Block as BlockT>::Hash>) -> Result<Vec<OrderbookRpc>> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(

            self.client.info().best_hash);


        let runtime_api_result = api.get_all_orderbook(&at);
        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI),
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }

    fn get_market_info(&self, _at: Option<<Block as BlockT>::Hash>, trading_pair: H256, blocknum: u32) -> Result<MarketDataRpc> {
        let api = self.client.runtime_api();
        let at = BlockId::hash(
            // Always take the best block hash for this RPC
            self.client.info().best_hash);

        let runtime_api_result = api.get_market_info(&at, trading_pair, blocknum);
        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI),
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }

    fn get_orderbook_updates(&self, _: Option<<Block as BlockT>::Hash>, trading_pair: H256) -> Result<OrderbookUpdates> {
        let api = self.client.runtime_api();

        let runtime_api_result = api.get_orderbook_updates(&BlockId::hash(self.client.info().best_hash),trading_pair);
        let temp = match runtime_api_result {
            Ok(x) => match x {
                Ok(z) => Ok(z),
                Err(x) => Err(x),
            }
            Err(_) => Err(ErrorRpc::ServerErrorWhileCallingAPI),
        };
        temp.map_err(|e| ErrorConvert::covert_to_rpc_error(e))
    }
}
