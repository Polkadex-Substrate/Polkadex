use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::{bounded::BoundedVec, ecdsa::Signature, sr25519, ConstU32, H256};

use bls_primitives::Public;
use parachain_polkadex_runtime::Runtime;
use substrate_api_client::{Api, MultiAddress, PlainTipExtrinsicParams, WsRpcClient};
use thea_primitives::types::Message;

use crate::{connector::traits::ForeignConnector, error::Error, types::GossipMessage};

pub struct ParachainClient {
	api: Arc<Api<sr25519::Pair, WsRpcClient, PlainTipExtrinsicParams<Runtime>, Runtime>>,
}

#[async_trait]
impl ForeignConnector for ParachainClient {
	fn block_duration() -> Duration {
		todo!()
	}

	async fn connect(url: String) -> Result<Self, Error> {
		let polkadex_client = WsRpcClient::new(url.as_str());
		let api = Arc::new(Api::<
			sr25519::Pair,
			WsRpcClient,
			PlainTipExtrinsicParams<Runtime>,
			Runtime,
		>::new(polkadex_client)?);
		Ok(ParachainClient { api })
	}

	async fn read_events(&self, block_num: u64) -> Result<Message, Error> {
		let block_hash: H256 = self
			.api
			.get_block_hash(Some(block_num.saturated_into()))?
			.ok_or(Error::BlockHashNotFound)?;
		// Read thea messages from foreign chain
		let incoming_message = self
			.api
			.get_storage_value::<Message>("", "OutgoingMessages", Some(block_hash))?
			.ok_or(Error::ErrorReadingTheaMessage)?;
		Ok(incoming_message)
	}

	async fn send_transaction(&self, message: GossipMessage) {
		todo!()
	}

	async fn last_finalized_block_number(&self) -> Result<u64, Error> {
		todo!()
	}
}
