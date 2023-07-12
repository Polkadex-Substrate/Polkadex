use parity_scale_codec::Decode;
use serde::{Deserialize, Serialize};
use sp_core::offchain::{Duration, HttpError};
use sp_runtime::offchain::{
	http,
	http::{Error, PendingRequest, Request, Response},
};
use sp_std::vec::Vec;
use thea_primitives::Network;

use crate::{Config, Pallet};

pub const MAINNET_URL: &str = "http://localhost:9944";
pub const PARACHAIN_URL: &str = "http://localhost:9933";

impl<T: Config> Pallet<T> {
	pub fn run_thea_validation(blk: T::BlockNumber) -> Result<(), &'static str> {
		// 1. Check for disputes and report if any and exit
		//		a. Get all payloads in dispute period
		//		b. Verify payload on the source chain on its finalized state
		//		c. if not verified, sign and submit dispute on-chain
		// 2. Check for new nonce to process for all networks
		//		a. Read the next nonce (N) to process at source and destination on its finalized state
		//		b. Check if payload for N is available at source and destination on its finalized state
		//		c. Compute who should sign this and if its us then sign the payload
		//		d. store the signed payload on-chain for relayers to relay it to destination
		Ok(())
	}

	pub fn get_storage_at_latest_finalized_head<S: Decode>(
		network: Network,
		storage_key: Vec<u8>,
	) -> Result<S, &'static str> {
		// 1. Get finalized head
		let finalized_head = Self::get_finalized_head(network)?;
		todo!()
	}

	pub fn get_finalized_head(network: Network) -> Result<Vec<u8>, &'static str> {
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5_000));

		// This body will work for most substrate chains
		let body = "{
				\"id\":1,
				\"jsonrpc\":\"2.0\",
				\"method\": \"chain_getFinalizedHead\",
				\"params\": []
		})";

		let request = match network {
			0 => http::Request::post(MAINNET_URL, [body]),
			1 => http::Request::post(PARACHAIN_URL, [body]),
			_ => return Err("Network not configured"),
		};
		log::debug!(target:"thea","Sending get_finalized_head request...");
		let pending: PendingRequest = request.deadline(deadline).send().map_err(map_http_err)?;

		log::debug!(target:"thea","Waiting for get_finalized_head response...");
		let response: Response = pending
			.try_wait(deadline)
			.map_err(|pending| "get_finalized_head deadline reached")?
			.map_err(map_sp_runtime_http_err)?;

		if response.code != 200u16 {
			log::warn!(target:"thea","Unexpected status code for get_finalized_head: {:?}",response.code);
			return Err("get_finalized_head request failed")
		}

		let body = response.body().collect::<Vec<u8>>();

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			"no UTF8 body in response"
		})?;
		log::debug!(target:"thea","get_finalized_head response: {:?}",body_str);
		let response: JSONRPCResponse = serde_json::from_str::<JSONRPCResponse>(&body_str)
			.map_err(|_| "Response failed deserialize")?;
		Ok(response.result)
	}
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
	jsonrpc: f32,
	result: Vec<u8>,
	id: u64,
}
