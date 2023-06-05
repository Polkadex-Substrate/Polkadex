// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! In this module defined concrete implementation of the foreign connector abstraction for the
//! parachain.

use std::time::Duration;

use async_trait::async_trait;
use log::info;
use parity_scale_codec::{Decode, Encode};
use subxt::{OnlineClient, PolkadotConfig};
use thea_primitives::types::Message;

use crate::{connector::traits::ForeignConnector, error::Error, types::GossipMessage};

#[subxt::subxt(runtime_metadata_path = "../../parachain-metadata/metadata.scale")]
mod parachain {}

/// Client to communicate with the parachain.
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
		// Get current validator set id
		let storage_address = parachain::storage().thea_message_handler().validator_set_id();
		// TODO: Get last finalized block hash
		let set_id = self
			.api
			.storage()
			.at_latest()
			.await
			.map_err(|err| {
				log::error!(target:"parachain","Error while fetching current set id: {:?}",err);
				err
			})?
			.fetch_or_default(&storage_address)
			.await
			.map_err(|err| {
				log::error!(target:"parachain","Error while fetching current set id: {:?}",err);
				err
			})?;

		// Get validator set
		let storage_address = parachain::storage().thea_message_handler().authorities(set_id);
		// TODO: Get last finalized block hash
		let auths = self
			.api
			.storage()
			.at_latest()
			.await
			.map_err(|err| {
				log::error!(target:"parachain","Error while fetching auth set: {:?}",err);
				err
			})?
			.fetch_or_default(&storage_address)
			.await
			.map_err(|err| {
				log::error!(target:"parachain","Error while fetching auth set {:?}",err);
				err
			})?;

		Ok(!auths.0.is_empty())
	}
}
