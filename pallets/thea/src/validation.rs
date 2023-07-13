use frame_system::offchain::SubmitTransaction;
use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Deserializer, Serialize};
use sp_core::offchain::{Duration, HttpError};
use sp_runtime::offchain::{
	http,
	http::{Error, PendingRequest, Request, Response},
};
use sp_std::{vec, vec::Vec};
use thea_primitives::{Message, Network};

use crate::{Call, Config, Pallet};

pub const MAINNET_URL: &str = "http://localhost:9944";
pub const PARACHAIN_URL: &str = "http://localhost:9933";
pub const AGGREGRATOR_URL: &str = "https://thea-aggregator.polkadex.trade";

impl<T: Config> Pallet<T> {
	pub fn run_thea_validation(blk: T::BlockNumber) -> Result<(), &'static str> {
		if !sp_io::offchain::is_validator() {
			return Ok(())
		}

		let active_networks = <ActiveNetworks<T>>::get();
		// 2. Check for new nonce to process for all networks
		for network in active_networks {
			//		a. Read the next nonce (N) to process at source and destination on its finalized
			// state
			let next_incoming_nonce = <IncomingNonce<T>>::get(network).saturating_add(1);
			let next_outgoing_nonce = <OutgoingNonce<T>>::get(network).saturating_add(1);
			//		b. Check if payload for N is available at source and destination on its finalized
			// state
			let next_incoming_message =
				get_payload_for_nonce(next_incoming_nonce, network, Destination::Parachain);
			let next_outgoing_message =
				get_payload_for_nonce(next_outgoing_nonce, network, Destination::Solochain);
			//		c. Compute who should sign this and if its us then sign the payload
			if let Some(message) = next_incoming_message {
				//  d. store the signed payload on-chain for relayers to relay it to destination
				compute_signer_and_submit::<T>(message, Destination::Solochain)?;
			}
			if let Some(message) = next_outgoing_message {
				compute_signer_and_submit::<T>(message, Destination::Parachain)?;
			}
		}
		Ok(())
	}
}

pub fn compute_signer_and_submit<T: Config>(
	message: Message,
	destination: Destination,
) -> Result<(), &'static str> {
	// We use SHA256 as it is available across many networks
	let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());

	let id = <ValidatorSetId<T>>::get();
	let authorities = <Authorities<T>>::get(id).to_vec();

	let local_keys = T::TheaId::all();

	let mut available_keys = authorities
		.into_iter()
		.enumerate()
		.filter_map(move |(_index, authority)| {
			local_keys
				.binary_search(&authority)
				.ok()
				.map(|location| local_keys[location].clone())
		})
		.collect::<Vec<T::TheaId>>();
	available_keys.sort();

	if available_keys.is_empty() {
		return Err("No active keys available")
	}

	let signer = available_keys.get(0).ok_or("Key not avaialble")?;
	// Note: this is a double hash signing
	let signature = signer.sign(&msg_hash).ok_or("Expected signature to be returned")?;

	submit_message_to_aggregator::<T>(message, signature.into(), destination)?;
	Ok(())
}

pub fn submit_message_to_aggregator<T: Config>(
	message: Message,
	signature: T::Signature,
	destination: Destination,
) -> Result<(), &'static str> {
	let body = serde_json::json!({
		"message": message,
		"signature": signature.encode(),
		"destination": destination
	});
	send_request(
		"thea_aggregator_link",
		AGGREGRATOR_URL,
		body.as_str().ok_or("Unable to create str")?,
	)?;
	Ok(())
}

pub fn get_payload_for_nonce(
	nonce: u64,
	network: Network,
	destination: Destination,
) -> Option<Message> {
	let mut message = None;

	match destination {
		Destination::Solochain => {
			// Get the outgoing message with nonce: `nonce` for network: `network`
			let key = create_solo_outgoing_message_key(nonce, network);
			message = get_storage_at_latest_finalized_head::<Option<Message>>(
				"solo_outgoing_message",
				MAINNET_URL,
				key,
			)
			.unwrap();
		},
		Destination::Parachain => {
			// Get the outgoing message with nonce: `nonce` from network
			let key = create_para_outgoing_message_key(nonce);
			message = get_storage_at_latest_finalized_head::<Option<Message>>(
				"para_outgoing_message",
				PARACHAIN_URL,
				key,
			)
			.unwrap();
		},
	}
	message
}

pub fn create_solo_outgoing_message_key(nonce: u64, network: Network) -> Vec<u8> {
	let module_name = sp_io::hashing::twox_128(b"Thea");
	let storage_prefix = sp_io::hashing::twox_128(b"OutgoingMessages");
	let mut key = Vec::new();
	key.append(&mut module_name.to_vec());
	key.append(&mut storage_prefix.to_vec());
	key.append(&mut network.encode());
	key.append(&mut nonce.encode());
	key
}

pub fn create_para_outgoing_message_key(nonce: u64) -> Vec<u8> {
	let module_name = sp_io::hashing::twox_128(b"TheaMessageHandler");
	let storage_prefix = sp_io::hashing::twox_128(b"OutgoingMessages");
	let mut key = Vec::new();
	key.append(&mut module_name.to_vec());
	key.append(&mut storage_prefix.to_vec());
	key.append(&mut nonce.encode());
	key
}

pub fn get_storage_at_latest_finalized_head<S: Decode>(
	log_target: &str,
	url: &str,
	storage_key: Vec<u8>,
) -> Result<S, &'static str> {
	log::debug!(target:"thea","getting storage for {}",log_target);
	// 1. Get finalized head ( Fh )
	let finalized_head = get_finalized_head(url)?;
	// 2. Get the storage at Fh
	let body = serde_json::json!({
	"id":1,
	"jsonrpc":"2.0",
	"method": "state_getStorage",
	"params": [storage_key,finalized_head]
	});

	let storage_bytes = send_request(
		log_target,
		url,
		body.as_str().unwrap(), // TODO: Remove unwraps
	)?;

	Ok(Decode::decode(&mut &storage_bytes[..]).map_err(|_| "Decode failure")?)
}

pub fn get_finalized_head<'a>(url: &str) -> Result<Vec<u8>, &'static str> {
	// This body will work for most substrate chains
	let body = serde_json::json!({
	"id":1,
	"jsonrpc":"2.0",
	"method": "chain_getFinalizedHead",
	"params": []
	});
	let result = send_request("get_finalized_head", url, body.to_string().as_str())?;
	Ok(result)
}
use crate::pallet::{ActiveNetworks, Authorities, IncomingNonce, OutgoingNonce, ValidatorSetId};
use parity_scale_codec::alloc::string::ToString;
use polkadex_primitives::Signature;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{bytes::to_hex, U256};
use thea_primitives::types::Destination;

pub fn send_request<'a>(log_target: &str, url: &str, body: &str) -> Result<Vec<u8>, &'static str> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5_000));

	let body_len = serde_json::to_string(&body.as_bytes().len()).unwrap();
	log::debug!(target:"thea","Sending {} request with body len {}...",log_target,body_len);
	let request = http::Request::post(url, [body]);
	let pending: PendingRequest = request
		.add_header("Content-Type", "application/json")
		.add_header("Content-Length", body_len.as_str())
		.deadline(deadline)
		.send()
		.map_err(map_http_err)?;

	log::debug!(target:"thea","Waiting for {} response...",log_target);
	let response: Response = pending
		.try_wait(deadline)
		.map_err(|pending| "deadline reached")?
		.map_err(map_sp_runtime_http_err)?;

	if response.code != 200u16 {
		log::warn!(target:"thea","Unexpected status code for {}: {:?}",log_target,response.code);
		return Err("request failed")
	}

	let body = response.body().collect::<Vec<u8>>();

	// Create a str slice from the body.
	let body_str = sp_std::str::from_utf8(body.as_slice()).map_err(|_| {
		log::warn!("No UTF8 body");
		"no UTF8 body in response"
	})?;
	log::debug!(target:"thea","{} response: {:?}",log_target,body_str);
	let response: JSONRPCResponse = serde_json::from_str::<JSONRPCResponse>(&body_str)
		.map_err(|_| "Response failed deserialize")?;
	Ok(response.result.clone())
}

fn map_sp_runtime_http_err(err: sp_runtime::offchain::http::Error) -> &'static str {
	match err {
		Error::DeadlineReached => "Deadline Reached",
		Error::IoError => "Io Error",
		Error::Unknown => "Unknown error",
	}
}
fn map_http_err(err: HttpError) -> &'static str {
	match err {
		HttpError::DeadlineReached => "Deadline Reached",
		HttpError::IoError => "Io Error",
		HttpError::Invalid => "Invalid request",
	}
}

#[derive(Serialize, Deserialize)]
pub struct JSONRPCResponse {
	jsonrpc: serde_json::Value,
	#[serde(deserialize_with = "deserialize_bytes")]
	result: Vec<u8>,
	id: u64,
}

fn deserialize_bytes<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
	D: Deserializer<'de>,
{
	// Deserialize the value as a string
	let s: &str = Deserialize::deserialize(deserializer)?;

	// Convert the string to bytes
	let bytes = s.as_bytes().to_vec();

	Ok(bytes)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::validation::get_finalized_head;
	use sp_io::TestExternalities;
	use sp_runtime::offchain::{testing, OffchainDbExt, OffchainWorkerExt};
	#[test]
	pub fn test_get_finalized_head() {
		env_logger::init();
		let (offchain, state) = testing::TestOffchainExt::new();
		let mut t = TestExternalities::default();
		t.register_extension(OffchainWorkerExt::new(offchain.clone()));
		// t.register_extension(OffchainDbExt::new(offchain.clone()));
		t.execute_with(|| {
			let head = get_finalized_head("https://mainnet.polkadex.trade").unwrap();
			println!("head: {:?}", hex::encode(head));
		});
	}
}
