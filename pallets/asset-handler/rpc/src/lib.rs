use std::{marker::PhantomData, sync::Arc};

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
pub use pallet_asset_handler_runtime_api::PolkadexAssetHandlerRuntimeApi;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, Header as HeaderT},
};

const RUNTIME_ERROR: i32 = 1;


#[rpc(client, server)]
pub trait PolkadexAssetHandlerRpcApi<BlockHash, AccountId, Hash> {
	#[method(name = "pallet_asset_handler_account_balances")]
	fn account_balances(
		&self,
		assets : Vec<u128>, 
        account_id : AccountId,
		at: Option<BlockHash>,
	) -> RpcResult<Vec<u128>>;
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
		assets : Vec<u128>, 
        account_id : AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<u128>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

		api.account_balances(&at, assets, account_id)
			.map_err(runtime_error_into_rpc_err)
	}
}

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		RUNTIME_ERROR,
		"Runtime error",
		Some(format!("{:?}", err)),
	))
	.into()
}
