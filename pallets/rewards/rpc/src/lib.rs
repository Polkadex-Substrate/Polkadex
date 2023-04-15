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
	#[method(name = "rewards_accountinfo")]
	fn account_info(
		&self,
		account_id: AccountId,
		reward_id: u32,
		at: Option<BlockHash>,
	) -> RpcResult<String>;
}

pub struct PolkadexRewardsRpc<Client, Block> {
	client: Arc<Client>,
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
