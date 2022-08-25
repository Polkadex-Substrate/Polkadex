use std::{marker::PhantomData, sync::Arc};

use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorCode, ErrorObject},
};
pub use pallet_ocex_runtime_api::PolkadexOcexRuntimeApi;
use pallet_ocex_primitives::WithdrawalWithPrimitives;
use sp_runtime::{
	generic::BlockId,
	traits::{Block as BlockT, Header as HeaderT},
};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_rpc::number::NumberOrHex;

const RUNTIME_ERROR: i32 = 1;

/// OCEX RPC methods.
#[rpc(client, server)]
pub trait PolkadexOcexRpcApi<BlockHash, AccountId, Hash>
{
    #[method(name = "pallet_ocex_return_withdrawals")]
    fn return_withdrawals(&self, snapshot_ids: Vec<u32>,account: AccountId, at: Option<BlockHash>) -> RpcResult<Vec<WithdrawalWithPrimitives<AccountId>>>;
}

pub struct PolkadexOcexRpc<Client, Block> {
	client: Arc<Client>,
	_marker: std::marker::PhantomData<Block>,
}

impl<Client, Block> PolkadexOcexRpc<Client, Block> {
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<Client, Block, AccountId, Hash>
	PolkadexOcexRpcApiServer<
		<Block as BlockT>::Hash,
		AccountId,
		Hash,
	> for PolkadexOcexRpc<Client, Block>
where
	Block: BlockT,
	Client: Send + Sync + 'static + ProvideRuntimeApi<Block> + HeaderBackend<Block>,
	Client::Api: PolkadexOcexRuntimeApi<
		Block,
		AccountId,
		Hash,
	>,
	AccountId: Codec,
	Hash: Codec,
{
    fn return_withdrawals(
		&self,
        snapshot_ids: Vec<u32>,
		account: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<WithdrawalWithPrimitives<AccountId>>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

		api.return_withdrawals(&at, snapshot_ids, account).map_err(runtime_error_into_rpc_err)
	}

}
/* 
fn return_withdrawals(
		&self,
        snapshot_ids: Vec<u32>,
		account: AccountId,
		at: Option<<Block as BlockT>::Hash>,
	) -> RpcResult<Vec<WithdrawalWithPrimitives<AccountId>>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(||
            // If the block hash is not supplied assume the best block.
            self.client.info().best_hash));

		api.return_withdrawals(&at, snapshot_ids, account).map_err(runtime_error_into_rpc_err)
	}
*/

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		RUNTIME_ERROR,
		"Runtime error",
		Some(format!("{:?}", err)),
	))
	.into()
}
