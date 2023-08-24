use crate::{
	pallet::{ActiveNetworks, Authorities, IncomingNonce, ValidatorSetId},
	Config, Pallet,
};
use parity_scale_codec::{alloc::string::ToString, Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_core::offchain::{Duration, HttpError};
use sp_runtime::offchain::{
	http,
	http::{Error, PendingRequest, Response},
};
use sp_std::{borrow::ToOwned, vec, vec::Vec};
use thea_primitives::{Message, Network};

use sp_application_crypto::RuntimeAppPublic;

use thea_primitives::types::{ApprovedMessage, Destination};

const MAINNET_URL: &str = "https://mainnet.polkadex.trade";
const PARACHAIN_URL: &str = "https://parachain.polkadex.trade";
const AGGREGRATOR_URL: &str = "https://thea.aggregator.polkadex.trade";

impl<T: Config> Pallet<T> {
	/// Starts the offchain worker instance that checks for finalized next incoming messages
	/// for both solochain and parachain, signs it and submits to aggregator
	pub fn run_thea_validation(_blk: T::BlockNumber) -> Result<(), &'static str> {
		if !sp_io::offchain::is_validator() {
			return Ok(())
		}

		let active_networks = <ActiveNetworks<T>>::get();
		log::debug!(target:"thea","List of active networks: {:?}",active_networks);
		// 2. Check for new nonce to process for all networks
		for network in active_networks {
			//		a. Read the next nonce (N) to process at source and destination on its finalized
			// state
			let next_incoming_nonce = <IncomingNonce<T>>::get(network).saturating_add(1);
			let next_outgoing_nonce = get_latest_incoming_nonce_parachain().saturating_add(1);
			log::debug!(target:"thea","Next Incoming nonce: {:?}, Outgoing nonce: {:?} for network: {:?}",
				next_incoming_nonce,next_outgoing_nonce,network);
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
			} else {
				log::debug!(target:"thea","No incoming message with nonce: {:?} from network: {:?}",next_incoming_nonce,network);
			}
			if let Some(message) = next_outgoing_message {
				compute_signer_and_submit::<T>(message, Destination::Parachain)?;
			} else {
				log::debug!(target:"thea","No outgoing message with nonce: {:?} to network: {:?}",next_outgoing_nonce,network);
			}
		}
		log::debug!(target:"thea","Thea offchain worker exiting..");
		Ok(())
	}
}

fn compute_signer_and_submit<T: Config>(
	message: Message,
	destination: Destination,
) -> Result<(), &'static str> {
	log::debug!(target:"thea","signing and submitting {:?} to {:?}",message, destination);
	// We use SHA256 as it is available across many networks
	let msg_hash = sp_io::hashing::sha2_256(message.encode().as_slice());

	let id = <ValidatorSetId<T>>::get();
	let authorities = <Authorities<T>>::get(id).to_vec();

	let local_keys = T::TheaId::all();

	let mut available_keys = authorities
		.iter()
		.enumerate()
		.filter_map(move |(_index, authority)| {
			local_keys
				.binary_search(authority)
				.ok()
				.map(|location| local_keys[location].clone())
		})
		.collect::<Vec<T::TheaId>>();
	available_keys.sort();

	if available_keys.is_empty() {
		return Err("No active keys available")
	}

	let signer = available_keys.get(0).ok_or("Key not avaialble")?;
	let mut auth_index = -1;
	for (index, auth) in authorities.iter().enumerate() {
		if auth == signer {
			auth_index = index as i32
		}
	}
	if auth_index < 0 {
		return Err("Unable to calculate auth index")
	}
	// Note: this is a double hash signing
	let signature = signer.sign(&msg_hash).ok_or("Expected signature to be returned")?;

	submit_message_to_aggregator::<T>(message, signature.into(), destination, auth_index as u16)?;
	Ok(())
}

fn submit_message_to_aggregator<T: Config>(
	message: Message,
	signature: T::Signature,
	destination: Destination,
	auth_index: u16,
) -> Result<(), &'static str> {
	log::debug!(target:"thea","submitting ({:?},{:?}) to aggregator",message.nonce,destination);
	let approved_message =
		ApprovedMessage { message, index: auth_index, signature: signature.encode(), destination };
	let body = serde_json::to_string(&approved_message).map_err(|err| {
		log::error!(target:"thea","Error serializing approved message: {:?}",err);
		"Error serializing approved message"
	})?;
	send_request("thea_aggregator_link", Destination::Aggregator, body.as_str())?;
	Ok(())
}

fn get_latest_incoming_nonce_parachain() -> u64 {
	let storage_key = create_para_incoming_nonce_key();
	get_storage_at_latest_finalized_head::<u64>(
		"para_incoming_nonce",
		Destination::Parachain,
		storage_key,
	)
	.unwrap_or_default()
	.unwrap_or_default()
}

fn get_payload_for_nonce(
	nonce: u64,
	network: Network,
	destination: Destination,
) -> Option<Message> {
	log::debug!(target:"thea","Getting payload for nonce {} for network: {} ,dest: {:?}",nonce,network,destination);
	match destination {
		Destination::Solochain => {
			// Get the outgoing message with nonce: `nonce` for network: `network`
			let key = create_solo_outgoing_message_key(nonce, network);
			match get_storage_at_latest_finalized_head::<Message>(
				"solo_outgoing_message",
				destination,
				key,
			) {
				Ok(message) => message,
				Err(err) => {
					log::error!(target:"thea","Unable to get finalized solo head: {:?}",err);
					None
				},
			}
		},
		Destination::Parachain => {
			// Get the outgoing message with nonce: `nonce` from network
			let key = create_para_outgoing_message_key(nonce);
			match get_storage_at_latest_finalized_head::<Message>(
				"para_outgoing_message",
				destination,
				key,
			) {
				Ok(message) => message,
				Err(err) => {
					log::error!(target:"thea","Unable to get finalized solo head: {:?}",err);
					None
				},
			}
		},
		_ => {
			log::warn!(target:"thea","Invalid destination provided");
			None
		},
	}
}

/// Returns the encoded key of the storage that stores incoming nonce on TheaMessageHandler pallet
fn create_para_incoming_nonce_key() -> Vec<u8> {
	let module_name = sp_io::hashing::twox_128(b"TheaMessageHandler");
	let storage_prefix = sp_io::hashing::twox_128(b"IncomingNonce");
	let mut key = Vec::new();
	key.append(&mut module_name.to_vec());
	key.append(&mut storage_prefix.to_vec());
	key
}
/// Returns the encoded key of the storage that stores outgoing nonce on Thea pallet
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
/// Returns the encoded key for outgoing message for given nonce in TheaMessageHandler pallet
pub fn create_para_outgoing_message_key(nonce: u64) -> Vec<u8> {
	let module_name = sp_io::hashing::twox_128(b"TheaMessageHandler");
	let storage_prefix = sp_io::hashing::twox_128(b"OutgoingMessages");
	let mut key = Vec::new();
	key.append(&mut module_name.to_vec());
	key.append(&mut storage_prefix.to_vec());
	key.append(&mut nonce.encode());
	key
}

fn get_storage_at_latest_finalized_head<S: Decode>(
	log_target: &str,
	destination: Destination,
	storage_key: Vec<u8>,
) -> Result<Option<S>, &'static str> {
	log::debug!(target:"thea","getting storage for {}",log_target);
	// 1. Get finalized head ( Fh )
	let finalized_head = get_finalized_head(destination)?;

	let storage_key = "0x".to_owned() + &hex::encode(storage_key);

	// 2. Get the storage at Fh
	let body = serde_json::json!({
	"id":1,
	"jsonrpc":"2.0",
	"method": "state_getStorage",
	"params": [storage_key,finalized_head]
	})
	.to_string();

	let storage_bytes = send_request(log_target, destination, body.as_str())?;

	if storage_bytes.is_null() {
		log::debug!(target:"thea","Storage query returned null response");
		return Ok(None)
	}

	let storage_bytes = storage_bytes.to_string().replace('\"', ""); // Remove unwanted \"
	let storage_bytes = storage_bytes.replace("0x", ""); // Remove unwanted 0x for decoding
	let storage_bytes =
		hex::decode(storage_bytes).map_err(|_| "Unable to hex decode storage value bytes")?;

	Ok(Some(Decode::decode(&mut &storage_bytes[..]).map_err(|_| "Decode failure")?))
}
use scale_info::prelude::string::String;

fn get_finalized_head(destination: Destination) -> Result<String, &'static str> {
	// This body will work for most substrate chains
	let body = serde_json::json!({
	"id":1,
	"jsonrpc":"2.0",
	"method": "chain_getFinalizedHead",
	"params": []
	});
	let mut result =
		send_request("get_finalized_head", destination, body.to_string().as_str())?.to_string();
	result = result.replace('\"', "");
	log::debug!(target:"thea","Finalized head: {:?}",result);
	Ok(result)
}

/// Returns the url based on destination and counter
///
/// Counter value 0 means return the local address
/// Counter value 1 means returns the fallback address.
pub fn resolve_destination_url(destination: Destination, counter: i32) -> String {
	if destination == Destination::Aggregator {
		return AGGREGRATOR_URL.to_string()
	}
	let url = match (destination, counter) {
		(Destination::Solochain, 0) => "http://localhost:9944",
		(Destination::Solochain, 1) => MAINNET_URL,
		(Destination::Parachain, 0) => "http://localhost:8844",
		(Destination::Parachain, 1) => PARACHAIN_URL,
		_ => AGGREGRATOR_URL,
	};
	log::debug!(target:"thea","Resolving {:?}: {:?} to {:?}",destination,counter,url);
	url.to_string()
}

pub fn send_request(
	log_target: &str,
	destination: Destination,
	body: &str,
) -> Result<serde_json::Value, &'static str> {
	for try_counter in 0..2 {
		match create_and_send_request(
			log_target,
			body,
			&resolve_destination_url(destination, try_counter),
		) {
			Ok(value) => return Ok(value),
			Err(err) => {
				log::error!(target:"thea","Error querying {:?}: {:?}",log_target, err);
			},
		}
	}
	Err("request failed")
}

fn create_and_send_request(
	log_target: &str,
	body: &str,
	url: &str,
) -> Result<serde_json::Value, &'static str> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(12_000));

	let body_len = serde_json::to_string(&body.as_bytes().len())
		.map_err(|_| "Unable to to string body len")?;
	log::debug!(target:"thea","Sending {} request with body len {}...",log_target,body_len);
	log::debug!(target:"thea","Sending {} request with body {}",log_target,body);
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
		.map_err(|_pending| "deadline reached")?
		.map_err(map_sp_runtime_http_err)?;

	if response.code != 200u16 {
		log::warn!(target:"thea","Unexpected status code for {}: {:?}",log_target,response.code);
		return Err("Unexpected status code")
	}

	let body = response.body().collect::<Vec<u8>>();

	// Create a str slice from the body.
	let body_str = sp_std::str::from_utf8(body.as_slice()).map_err(|_| {
		log::warn!("No UTF8 body");
		"no UTF8 body in response"
	})?;
	log::debug!(target:"thea","{} response: {:?}",log_target,body_str);
	let response: JSONRPCResponse = serde_json::from_str::<JSONRPCResponse>(body_str)
		.map_err(|_| "Response failed deserialize")?;
	Ok(response.result)
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

/// Http Resposne body
#[derive(Serialize, Deserialize)]
pub struct JSONRPCResponse {
	jsonrpc: serde_json::Value,
	result: serde_json::Value,
	id: u64,
}

impl JSONRPCResponse {
	pub fn new(content: Vec<u8>) -> Self {
		Self { jsonrpc: "2.0".into(), result: content.into(), id: 2 }
	}
}
