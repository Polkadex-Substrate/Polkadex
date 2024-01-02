// This file is part of Polkadex.
//
// Copyright (c) 2022-2023 Polkadex oü.
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

use crate::{validation::JSONRPCResponse, Authorities, Config, ValidatorSetId};
use parity_scale_codec::{alloc::string::ToString, Encode};
use scale_info::prelude::string::String;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::offchain::{Duration, HttpError};
use sp_runtime::offchain::{
	http,
	http::{Error, PendingRequest, Response},
};
use sp_std::{marker::PhantomData, vec::Vec};
use thea_primitives::{
	types::{ApprovedMessage, Destination},
	Message,
};

const MAINNET_URL: &str = "http://localhost:10000";
const PARACHAIN_URL: &str = "http://localhost:8845";
const AGGREGRATOR_URL: &str = "http://localhost:9001";

pub struct Resolver<T: Config>(pub PhantomData<T>);

impl<T: Config> Resolver<T> {
	/// Generate request body for the given message and send it to the destination
	/// # Parameters
	/// * `log_target`: Log target for the request
	/// * `destination`: Destination to which request should be sent
	/// * `body`: Body of the request
	pub fn send_request(
		log_target: &str,
		destination: Destination,
		body: &str,
	) -> Result<serde_json::Value, &'static str> {
		for try_counter in 0..2 {
			match Self::create_and_send_request(
				log_target,
				body,
				&Self::resolve_destination_url(destination, try_counter),
			) {
				Ok(value) => return Ok(value),
				Err(err) => {
					log::error!(target:"thea","Error querying {:?}: {:?}",log_target, err);
				},
			}
		}
		Err("request failed")
	}

	///
	pub(crate) fn compute_signer_and_submit(
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

		let signer = available_keys.first().ok_or("Key not avaialble")?;
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

		Self::submit_message_to_aggregator(
			message,
			signature.into(),
			destination,
			auth_index as u16,
		)?;
		Ok(())
	}

	/// Submit message to aggregator
	/// # Parameters
	/// * `message`: Message to submit
	/// * `signature`: Signed Message
	/// * `destination`: Destination to which request should be sent
	/// * `auth_index`: Index of the signer in the authorities
	/// # Returns
	/// * `Result<(), &'static str>`: Ok if message was submitted successfully, error message
	///   otherwise
	fn submit_message_to_aggregator(
		message: Message,
		signature: T::Signature,
		destination: Destination,
		auth_index: u16,
	) -> Result<(), &'static str> {
		log::debug!(target:"thea","submitting ({:?},{:?}) to aggregator",message.nonce,destination);
		let approved_message = ApprovedMessage {
			message,
			index: auth_index,
			signature: signature.encode(),
			destination,
		};
		let body = serde_json::to_string(&approved_message).map_err(|err| {
			log::error!(target:"thea","Error serializing approved message: {:?}",err);
			"Error serializing approved message"
		})?;
		Self::send_request("thea_aggregator_link", Destination::Aggregator, body.as_str())?;
		Ok(())
	}

	/// Create and send request to the given url
	/// # Parameters
	/// * `log_target`: Log target for the request
	/// * `body`: Body of the request
	/// * `url`: Url to send request to
	/// # Returns
	/// * `Result<serde_json::Value, &'static str>`: Response body or error message
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
			.map_err(Self::map_http_err)?;

		log::debug!(target:"thea","Waiting for {} response...",log_target);
		let response: Response = pending
			.try_wait(deadline)
			.map_err(|_pending| "deadline reached")?
			.map_err(Self::map_sp_runtime_http_err)?;

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

	/// Resolve destination url for the given destination
	/// # Parameters
	/// * `destination`: Destination to resolve
	/// * `counter`: Counter to resolve
	/// # Returns
	/// * `String`: Resolved url
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

	/// Map http error to static string
	/// # Parameters
	/// * `err`: Http error to map
	/// # Returns
	/// * `&'static str`: Mapped error
	fn map_http_err(err: HttpError) -> &'static str {
		match err {
			HttpError::DeadlineReached => "Deadline Reached",
			HttpError::IoError => "Io Error",
			HttpError::Invalid => "Invalid request",
		}
	}

	/// Map sp_runtime http error to static string
	/// # Parameters
	/// * `err`: Http error to map
	/// # Returns
	/// * `&'static str`: Mapped error
	fn map_sp_runtime_http_err(err: sp_runtime::offchain::http::Error) -> &'static str {
		match err {
			Error::DeadlineReached => "Deadline Reached",
			Error::IoError => "Io Error",
			Error::Unknown => "Unknown error",
		}
	}
}
