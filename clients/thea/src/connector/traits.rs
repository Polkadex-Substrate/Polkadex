use crate::{error::Error, types::GossipMessage};
use async_trait::async_trait;
use std::time::Duration;
use thea_primitives::types::Message;

#[async_trait]
pub trait ForeignConnector {
	/// Block duration
	fn block_duration(&self) -> Duration;
	/// Initialize the connection to native blockchain
	async fn connect(url: String) -> Result<Self, Error>
	where
		Self: Sized;
	/// Read all interested events based on the last processed nonce of that network on Polkadex
	async fn read_events(&self, last_processed_nonce: u64) -> Result<Option<Message>, Error>;
	/// Sends transaction to blockchain, if failed, retry every second until its successful
	async fn send_transaction(&self, message: GossipMessage);
}
