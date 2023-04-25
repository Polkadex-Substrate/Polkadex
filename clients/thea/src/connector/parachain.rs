use std::time::Duration;

use async_trait::async_trait;
use parity_scale_codec::Encode;
use subxt::{dynamic::Value, storage::DynamicAddress, OnlineClient, PolkadotConfig};
use thea_primitives::types::Message;

use crate::{connector::traits::ForeignConnector, error::Error, types::GossipMessage};

pub struct ParachainClient {
	api: OnlineClient<PolkadotConfig>,
}

// This should not be changed unless name at parachain-runtime/lib is changed.
const PALLET_NAME: &str = "TheaMessageHandler";

#[async_trait]
impl ForeignConnector for ParachainClient {
	fn block_duration(&self) -> Duration {
		// Parachain block time is 12 second , but we check every 10s to prevent drift
		Duration::from_secs(10)
	}

	async fn connect(url: String) -> Result<Self, Error> {
		let api = OnlineClient::<PolkadotConfig>::from_url(url).await?;
		Ok(ParachainClient { api })
	}

	async fn read_events(&self, last_processed_nonce: u64) -> Result<Option<Message>, Error> {
		// Read thea messages from foreign chain
		let storage_address = subxt::dynamic::storage(
			PALLET_NAME,
			"OutgoingMessages",
			vec![
				// Something that encodes to an AccountId32 is what we need for the map key here:
				Value::from_bytes(last_processed_nonce.saturating_add(1).encode()),
			],
		);
		// TODO: Get last finalized block hash
		let encoded_bytes = self
			.api
			.storage()
			.at_latest()
			.await?
			.fetch_or_default(&storage_address)
			.await?
			.into_encoded();

		Ok(parity_scale_codec::Decode::decode(&mut &encoded_bytes[..])?)
	}

	async fn send_transaction(&self, message: GossipMessage) {
		let call = subxt::dynamic::tx(
			PALLET_NAME,
			"incoming_message",
			vec![
				// Bitmap
				Value::from(message.bitmap.clone()),
				// Payload
				Value::from_bytes(message.payload.encode()),
				// Signature
				Value::from_bytes(message.aggregate_signature.encode()),
			],
		);

		self.api
			.tx()
			.create_unsigned(&call)
			.unwrap()
			.submit_and_watch()
			.await
			.unwrap()
			.wait_for_in_block()
			.await
			.unwrap();
	}

	async fn check_message(&self, message: &Message) -> Result<bool, Error> {
		// Read thea messages from foreign chain
		let storage_address = subxt::dynamic::storage(
			PALLET_NAME,
			"OutgoingMessages",
			vec![
				// Something that encodes to an AccountId32 is what we need for the map key here:
				Value::from_bytes(message.nonce.encode()),
			],
		);
		// TODO: Get last finalized block hash
		let encoded_bytes = self
			.api
			.storage()
			.at_latest()
			.await?
			.fetch_or_default(&storage_address)
			.await?
			.into_encoded();

		let message_option: Option<Message> =
			parity_scale_codec::Decode::decode(&mut &encoded_bytes[..])?;
		let message_from_chain = message_option.ok_or(Error::ErrorReadingTheaMessage)?;

		Ok(message_from_chain == message.clone())
	}

	async fn last_processed_nonce_from_native(&self) -> Result<u64, Error> {
		// Read native network nonce from foreign chain
		let storage_address: DynamicAddress<Value> =
			subxt::dynamic::storage(PALLET_NAME, "IncomingNonce", vec![]);
		// TODO: Get last finalized block hash
		let encoded_bytes = self
			.api
			.storage()
			.at_latest()
			.await?
			.fetch_or_default(&storage_address)
			.await?
			.into_encoded();

		let message_option: u64 = parity_scale_codec::Decode::decode(&mut &encoded_bytes[..])?;

		Ok(message_option)
	}
}
