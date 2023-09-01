use crate::{
	aggregator::AggregatorClient,
	pallet::{ActiveNetworks, IncomingNonce},
	resolver::Resolver,
	Config, Pallet,
};
use serde::{Deserialize, Serialize};
use sp_std::vec::Vec;
use thea_primitives::types::Destination;

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
			let next_outgoing_nonce =
				AggregatorClient::<u64, T>::get_latest_incoming_nonce_parachain().saturating_add(1);
			log::debug!(target:"thea","Next Incoming nonce: {:?}, Outgoing nonce: {:?} for network: {:?}",
				next_incoming_nonce,next_outgoing_nonce,network);
			//		b. Check if payload for N is available at source and destination on its finalized
			// state
			let next_incoming_message = AggregatorClient::<u64, T>::get_payload_for_nonce(
				next_incoming_nonce,
				network,
				Destination::Parachain,
			);
			let next_outgoing_message = AggregatorClient::<u64, T>::get_payload_for_nonce(
				next_outgoing_nonce,
				network,
				Destination::Solochain,
			);
   //		c. Compute who should sign this and if its us then sign the payload
			if let Some(message) = next_incoming_message {
				//  d. store the signed payload on-chain for relayers to relay it to destination
				Resolver::<T>::compute_signer_and_submit(message, Destination::Solochain)?;
				                                                            // Resolver Struct
				                                                            // object
			} else {
				log::debug!(target:"thea","No incoming message with nonce: {:?} from network: {:?}",next_incoming_nonce,network);
			}
			if let Some(message) = next_outgoing_message {
				Resolver::<T>::compute_signer_and_submit(message, Destination::Parachain)?;
				                                                            // Resolver Struct
				                                                            // object
			} else {
				log::debug!(target:"thea","No outgoing message with nonce: {:?} to network: {:?}",next_outgoing_nonce,network);
			}
		}
		log::debug!(target:"thea","Thea offchain worker exiting..");
		Ok(())
	}
}

/// Http Resposne body
#[derive(Serialize, Deserialize)]
pub struct JSONRPCResponse {
	jsonrpc: serde_json::Value,
	pub(crate) result: serde_json::Value,
	id: u64,
}

impl JSONRPCResponse {
	pub fn new(content: Vec<u8>) -> Self {
		Self { jsonrpc: "2.0".into(), result: content.into(), id: 2 }
	}
}
