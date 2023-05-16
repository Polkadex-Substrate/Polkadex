use std::time::Duration;

use async_trait::async_trait;
use log::info;
use parity_scale_codec::{Decode, Encode};
use subxt::{OnlineClient, PolkadotConfig};
use thea_primitives::{types::Message, AuthorityId};

use crate::{connector::traits::ForeignConnector, error::Error, types::GossipMessage};

#[subxt::subxt(runtime_metadata_path = "../../parachain-metadata/metadata.scale")]
mod parachain {}

pub struct ParachainClient {
	api: OnlineClient<PolkadotConfig>,
}

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

	async fn read_events(&self, nonce: u64) -> Result<Option<Message>, Error> {
		// Read thea messages from foreign chain
		let storage_address = parachain::storage().thea_message_handler().outgoing_messages(nonce);
		// TODO: Get last finalized block hash
		let encoded_bytes =
			self.api.storage().at_latest().await?.fetch(&storage_address).await?.encode();

		Ok(parity_scale_codec::Decode::decode(&mut &encoded_bytes[..])?)
	}

	async fn send_transaction(&self, message: GossipMessage) -> Result<(), Error> {
		info!(target:"thea", "Sending message to foreign runtime");
		let call = parachain::tx().thea_message_handler().incoming_message(
			message.bitmap,
			Decode::decode(&mut &message.payload.encode()[..])?,
			Decode::decode(&mut &message.aggregate_signature.encode()[..])?,
		);
		info!(target:"thea", "Tx created: {:?}",call);
		let tx_result = self
			.api
			.tx()
			.create_unsigned(&call)?
			.submit_and_watch()
			.await?
			.wait_for_in_block()
			.await?
			.wait_for_success()
			.await?;

		info!(target:"thea", "Tx included: {:?}",tx_result.block_hash());
		Ok(())
	}

	async fn check_message(&self, message: &Message) -> Result<bool, Error> {
		// Read thea messages from foreign chain
		let storage_address =
			parachain::storage().thea_message_handler().outgoing_messages(message.nonce);
		// TODO: Get last finalized block hash
		let encoded_bytes =
			self.api.storage().at_latest().await?.fetch(&storage_address).await?.encode();

		let message_option: Option<Message> =
			parity_scale_codec::Decode::decode(&mut &encoded_bytes[..])?;

		match message_option {
			None => return Ok(false),
			Some(message_from_chain) => Ok(message_from_chain == message.clone()),
		}
	}

	async fn last_processed_nonce_from_native(&self) -> Result<u64, Error> {
		// Read native network nonce from foreign chain
		let storage_address = parachain::storage().thea_message_handler().incoming_nonce();
		// TODO: Get last finalized block hash
		let nonce =
			self.api.storage().at_latest().await?.fetch_or_default(&storage_address).await?;
		Ok(nonce)
	}

	async fn check_thea_authority_initialization(&self) -> Result<bool, Error> {
		// Read the validator set
		let runtime_api_call = parachain::apis().thea_parachain_api().get_current_authorities();

		// Submit the call and get back a result.
		let auths: Vec<AuthorityId> = self
			.api
			.runtime_api()
			.at_latest()
			.await
			.map_err(|err| {
				log::error!(target:"parachain","Error while calling runtime api: {:?}",err);
				err
			})?
			.call(runtime_api_call)
			.await;

		Ok(!auths.is_empty())
	}
}
