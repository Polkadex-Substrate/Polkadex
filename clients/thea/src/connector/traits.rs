use crate::{error::Error, types::GossipMessage};
use async_trait::async_trait;
use std::time::Duration;

#[async_trait]
pub trait ForeignConnector {
	/// Block duration
	fn block_duration() -> Duration;
	/// Initialize the connection to native blockchain
	async fn connect(url: String) -> Result<Self, Error>
	where
		Self: Sized;
	/// Read all interested events for the given block
	async fn read_events(&self, block_num: u64) -> Result<Messasge, Error>;
	/// Sends transaction to blockchain, if failed, retry every second until its successful
	async fn send_transaction(&self, message: GossipMessage);
	/// Last finalized block number
	async fn last_finalized_block_number(&self) -> Result<u64, Error>;
}
