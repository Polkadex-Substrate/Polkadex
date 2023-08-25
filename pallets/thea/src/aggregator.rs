use crate::{resolver::Resolver, Config};
use parity_scale_codec::{alloc::string::ToString, Decode, Encode};
use scale_info::prelude::string::String;
use sp_std::{marker::PhantomData, prelude::ToOwned, vec, vec::Vec};
use thea_primitives::{types::Destination, Message, Network};

pub struct AggregatorClient<S: Decode, T: Config>(pub PhantomData<(S, T)>);

impl<S: Decode, T: Config> AggregatorClient<S, T> {
	/// Returns the latest incoming nonce for parachain
	/// # Returns
	/// * `u64`: Latest incoming nonce for parachain
	pub fn get_latest_incoming_nonce_parachain() -> u64 {
		let storage_key = Self::create_para_incoming_nonce_key();
		Self::get_storage_at_latest_finalized_head::<u64>(
			"para_incoming_nonce",
			Destination::Parachain,
			storage_key,
		)
		.unwrap_or_default()
		.unwrap_or_default()
	}

	/// Returns the latest incoming nonce for solochain
	/// # Returns
	/// * `u64`: Latest incoming nonce for solochain
	pub fn get_payload_for_nonce(
		nonce: u64,
		network: Network,
		destination: Destination,
	) -> Option<Message> {
		log::debug!(target:"thea","Getting payload for nonce {} for network: {} ,dest: {:?}",nonce,network,destination);
		match destination {
			Destination::Solochain => {
				// Get the outgoing message with nonce: `nonce` for network: `network`
				let key = Self::create_solo_outgoing_message_key(nonce, network);
				match Self::get_storage_at_latest_finalized_head::<Message>(
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
				let key = Self::create_para_outgoing_message_key(nonce);
				match Self::get_storage_at_latest_finalized_head::<Message>(
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

	/// Returns the encoded key for outgoing message for given nonce
	/// # Parameters
	/// * `nonce`: Nonce of the outgoing message
	/// * `network`: Network of the outgoing message
	/// # Returns
	/// * `Vec<u8>`: Encoded key for outgoing message for given nonce
	fn create_solo_outgoing_message_key(nonce: u64, network: Network) -> Vec<u8> {
		let module_name = sp_io::hashing::twox_128(b"Thea");
		let storage_prefix = sp_io::hashing::twox_128(b"OutgoingMessages");
		let mut key = Vec::new();
		key.append(&mut module_name.to_vec());
		key.append(&mut storage_prefix.to_vec());
		key.append(&mut network.encode());
		key.append(&mut nonce.encode());
		key
	}

	/// Returns the encoded key for outgoing message for given nonce for parachain
	/// # Parameters
	/// * `nonce`: Nonce of the outgoing message
	/// # Returns
	/// * `Vec<u8>`: Encoded key for outgoing message for given nonce for parachain
	pub fn create_para_outgoing_message_key(nonce: u64) -> Vec<u8> {
		let module_name = sp_io::hashing::twox_128(b"TheaMessageHandler");
		let storage_prefix = sp_io::hashing::twox_128(b"OutgoingMessages");
		let mut key = Vec::new();
		key.append(&mut module_name.to_vec());
		key.append(&mut storage_prefix.to_vec());
		key.append(&mut nonce.encode());
		key
	}

	/// Returns the encoded key for incoming nonce for parachain
	/// # Returns
	/// * `Vec<u8>`: Encoded key for incoming nonce for parachain
	fn create_para_incoming_nonce_key() -> Vec<u8> {
		let module_name = sp_io::hashing::twox_128(b"TheaMessageHandler");
		let storage_prefix = sp_io::hashing::twox_128(b"IncomingNonce");
		let mut key = Vec::new();
		key.append(&mut module_name.to_vec());
		key.append(&mut storage_prefix.to_vec());
		key
	}

	/// Returns the storage value for given key at latest finalized head
	/// # Parameters
	/// * `log_target`: Log target for debug logs
	/// * `destination`: Message destination
	fn get_storage_at_latest_finalized_head<A: Decode>(
		log_target: &str,
		destination: Destination,
		storage_key: Vec<u8>,
	) -> Result<Option<A>, &'static str> {
		log::debug!(target:"thea","getting storage for {}",log_target);
		// 1. Get finalized head ( Fh )
		let finalized_head = Self::get_finalized_head(destination)?;
		let storage_key = "0x".to_owned() + &hex::encode(storage_key);
		let body = serde_json::json!({
		"id":1,
		"jsonrpc":"2.0",
		"method": "state_getStorage",
		"params": [storage_key,finalized_head]
		})
		.to_string();
		let storage_bytes = Resolver::<T>::send_request(log_target, destination, body.as_str())?;
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

	/// Returns the latest finalized head
	/// # Parameters
	/// * `destination`: Message destination
	fn get_finalized_head(destination: Destination) -> Result<String, &'static str> {
		let body = serde_json::json!({
		"id":1,
		"jsonrpc":"2.0",
		"method": "chain_getFinalizedHead",
		"params": []
		});
		let mut result = Resolver::<T>::send_request(
			"get_finalized_head",
			destination,
			body.to_string().as_str(),
		)?
		.to_string();
		result = result.replace('\"', "");
		log::debug!(target:"thea","Finalized head: {:?}",result);
		Ok(result)
	}
}
