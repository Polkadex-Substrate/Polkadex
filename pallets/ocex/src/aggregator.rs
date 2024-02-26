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

use crate::{
	validator::{JSONRPCResponse, AGGREGATOR, LAST_PROCESSED_SNAPSHOT},
	Config,
};
use orderbook_primitives::{
	types::{ApprovedSnapshot, UserActionBatch},
	ObCheckpointRaw, SnapshotSummary,
};
use parity_scale_codec::{alloc::string::ToString, Decode, Encode};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::offchain::{Duration, HttpError};
use sp_runtime::{
	offchain::{
		http,
		http::{Error, PendingRequest, Response},
		storage::StorageValueRef,
	},
	SaturatedConversion,
};
use sp_std::{marker::PhantomData, prelude::ToOwned, vec::Vec};

pub struct AggregatorClient<T: Config>(PhantomData<T>);

impl<T: Config> AggregatorClient<T> {
	/// Load signed summary and send it to the aggregator
	/// # Parameters
	/// * `snapshot_id`: Snapshot id for which signed summary should be loaded and sent
	pub fn load_signed_summary_and_send(snapshot_id: u64) {
		let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
		key.append(&mut snapshot_id.encode());

		let summay_ref = StorageValueRef::persistent(&key);
		match summay_ref.get::<(
			SnapshotSummary<T::AccountId>,
			<<T as Config>::AuthorityId as RuntimeAppPublic>::Signature,
			u16,
		)>() {
			Ok(Some((summary, signature, index))) => {
				match serde_json::to_string(&ApprovedSnapshot {
					summary: summary.encode(),
					index: index.saturated_into(),
					signature: signature.encode(),
				}) {
					Ok(body) => {
						if let Err(err) = Self::send_request(
							"submit_snapshot_api",
							&(AGGREGATOR.to_owned() + "/submit_snapshot"),
							body.as_str(),
						) {
							log::error!(target:"ocex","Error submitting signature: {:?}",err);
						}
					},
					Err(err) => {
						log::error!(target:"ocex","Error serializing ApprovedSnapshot: {:?}",err);
					},
				}
			},
			Ok(None) => {
				log::error!(target:"ocex"," signed summary for:  nonce {:?} not found",snapshot_id);
			},
			Err(err) => {
				log::error!(target:"ocex","Error loading signed summary for:  nonce {:?}, {:?}",snapshot_id,err);
			},
		}
	}



	/// Load user action batch from aggregator
	/// # Parameters
	/// * `id`: Batch id to load
	/// # Returns
	/// * `Option<UserActionBatch<T::AccountId>>`: Loaded batch or None if error occured
	pub fn get_user_action_batch(id: u64) -> Option<UserActionBatch<T::AccountId>> {
		let body = serde_json::json!({ "id": id }).to_string();
		let result = match Self::send_request(
			"user_actions_batch",
			&(AGGREGATOR.to_owned() + "/snapshots"),
			&body,
		) {
			Ok(encoded_batch) => encoded_batch,
			Err(err) => {
				log::error!(target:"ocex","Error fetching user actions batch for {:?}: {:?}",id,err);
				return None;
			},
		};

		match UserActionBatch::<T::AccountId>::decode(&mut &result[..]) {
			Ok(batch) => Some(batch),
			Err(_) => {
				log::error!(target:"ocex","Unable to decode batch");
				None
			},
		}
	}

	/// Load checkpoint from aggregator
	/// # Returns
	/// * `Option<ObCheckpointRaw>`: Loaded checkpoint or None if error occured
	pub fn get_checkpoint() -> Option<ObCheckpointRaw> {
		let body = serde_json::json!({}).to_string();
		let result = match Self::send_request(
			"checkpoint",
			&(AGGREGATOR.to_owned() + "/latest_checkpoint"),
			&body,
		) {
			Ok(encoded_checkpoint) => encoded_checkpoint,
			Err(err) => {
				log::error!(target:"ocex","Error fetching checkpoint: {:?}",err);
				return None;
			},
		};

		match ObCheckpointRaw::decode(&mut &result[..]) {
			Ok(checkpoint) => Some(checkpoint),
			Err(_) => {
				log::error!(target:"ocex","Unable to decode checkpoint");
				None
			},
		}
	}

	/// Send request to aggregator
	/// # Parameters
	/// * `log_target`: Log target for debug logs
	/// * `url`: Url to send request to
	/// * `body`: Body of the request
	/// # Returns
	/// * `Result<Vec<u8>, &'static str>`: Response body or error message
	pub fn send_request(log_target: &str, url: &str, body: &str) -> Result<Vec<u8>, &'static str> {
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(12_000));

		let body_len =
			serde_json::to_string(&body.as_bytes().len()).map_err(|_| "Unable to serialize")?;
		log::debug!(target:"ocex","Sending {} request with body len {}...",log_target,body_len);
		let request = http::Request::post(url, [body]);
		let pending: PendingRequest = request
			.add_header("Content-Type", "application/json")
			.add_header("Content-Length", body_len.as_str())
			.deadline(deadline)
			.send()
			.map_err(Self::map_http_err)?;

		log::debug!(target:"ocex","Waiting for {} response...",log_target);
		let response: Response = pending
			.try_wait(deadline)
			.map_err(|_pending| "deadline reached")?
			.map_err(Self::map_sp_runtime_http_err)?;

		if response.code != 200u16 {
			log::warn!(target:"ocex","Unexpected status code for {}: {:?}",log_target,response.code);
			return Err("request failed");
		}

		let body = response.body().collect::<Vec<u8>>();

		// Create a str slice from the body.
		let body_str = sp_std::str::from_utf8(body.as_slice()).map_err(|_| {
			log::warn!("No UTF8 body");
			"no UTF8 body in response"
		})?;
		log::debug!(target:"ocex","{} response: {:?}",log_target,body_str);
		let response: JSONRPCResponse = serde_json::from_str::<JSONRPCResponse>(body_str)
			.map_err(|_| "Response failed deserialize")?;

		Ok(response.result)
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
