use crate::{
	pallet::{Accounts, ValidatorSetId},
	settlement::{add_balance, process_trade, sub_balance},
	snapshot::StateInfo,
	storage::store_trie_root,
	Config, Pallet, SnapshotNonce,
};

use orderbook_primitives::{
	types::{Trade, UserActionBatch, UserActions, WithdrawalRequest},
	SnapshotSummary,
};
use polkadex_primitives::{ingress::IngressMessages, withdrawal::Withdrawal};

use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{
	offchain::{Duration, HttpError},
	H256,
};
use sp_runtime::{
	offchain::{
		http,
		http::{Error, PendingRequest, Response},
		storage::StorageValueRef,
	},
	traits::BlakeTwo256,
	SaturatedConversion,
};
use sp_std::{boxed::Box, vec::Vec};
use sp_trie::{LayoutV1, TrieDBMut};
use trie_db::{TrieError, TrieMut};

pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";
const STATE_INFO: [u8; 25] = *b"offchain-ocex::state_info";
const LAST_PROCESSED_SNAPSHOT: [u8; 26] = *b"offchain-ocex::snapshot_id";

pub const AGGREGATOR: &str = "https://testnet.ob.aggregator.polkadex.trade";

impl<T: Config> Pallet<T> {
	pub fn run_on_chain_validation(_block_num: T::BlockNumber) -> Result<(), &'static str> {
		let local_keys = T::AuthorityId::all();
		let authorities = Self::validator_set().validators;
		let mut available_keys = authorities
			.iter()
			.enumerate()
			.filter_map(move |(_index, authority)| {
				local_keys
					.binary_search(authority)
					.ok()
					.map(|location| local_keys[location].clone())
			})
			.collect::<Vec<T::AuthorityId>>();

		available_keys.sort();

		if available_keys.is_empty() && sp_io::offchain::is_validator() {
			return Err("No active keys available")
		}

		// Check if another worker is already running or not
		let s_info = StorageValueRef::persistent(&WORKER_STATUS);
		match s_info.get::<bool>().map_err(|err| {
			log::error!(target:"ocex","Error while loading worker status: {:?}",err);
			"Unable to load worker status"
		})? {
			Some(true) => {
				// Another worker is online, so exit
				log::info!(target:"ocex", "Another worker is online, so exit");
				return Ok(())
			},
			None => {},
			Some(false) => {},
		}
		s_info.set(&true); // Set WORKER_STATUS to true
				   // Check the next batch to process
		let next_nonce = <SnapshotNonce<T>>::get().saturating_add(1);

		// Load the state to memory
		let mut root = crate::storage::load_trie_root();
		log::info!(target:"ocex","block: {:?}, state_root {:?}", _block_num, root);
		let mut storage = crate::storage::State;
		let mut state = crate::storage::get_state_trie(&mut storage, &mut root);

		let mut state_info = Self::load_state_info(&state);

		let last_processed_nonce = state_info.snapshot_id;

		// Check if we already processed this snapshot and updated our offchain state.
		if last_processed_nonce == next_nonce {
			log::debug!(target:"ocex","Submitting last processed snapshot: {:?}",next_nonce);
			// resubmit the summary to aggregator
			load_signed_summary_and_send::<T>(next_nonce);
			return Ok(())
		}

		log::info!(target:"ocex","last_processed_nonce: {:?}, next_nonce: {:?}",last_processed_nonce, next_nonce);

		if next_nonce.saturating_sub(last_processed_nonce) > 2 {
			// We need to sync our offchain state
			for nonce in last_processed_nonce.saturating_add(1)..next_nonce {
				// Load the next ObMessages
				let batch = match get_user_action_batch::<T>(nonce) {
					None => {
						log::error!(target:"ocex","No user actions found for nonce: {:?}",nonce);
						return Ok(())
					},
					Some(batch) => batch,
				};
				sp_runtime::print("Processing nonce");
				sp_runtime::print(nonce);
				Self::process_batch(&mut state, &batch, &mut state_info)?;
			}
		}

		// Load the next ObMessages
		log::info!(target:"ocex","Loading user actions for nonce: {:?}",next_nonce);
		let batch = match get_user_action_batch::<T>(next_nonce) {
			None => {
				log::debug!(target:"ocex","No user actions found for nonce: {:?}",next_nonce);
				// Store the last processed nonce
				// We need to -1 from next_nonce, as it is not yet processed
				state_info.snapshot_id = next_nonce.saturating_sub(1);
				Self::store_state_info(state_info, &mut state)?;
				state.commit();
				store_trie_root(*state.root());
				log::debug!(target:"ocex","Stored state root: {:?}",state.root());
				return Ok(())
			},
			Some(batch) => batch,
		};

		log::info!(target:"ocex","Processing user actions for nonce: {:?}",next_nonce);
		let withdrawals = Self::process_batch(&mut state, &batch, &mut state_info)?;

		if sp_io::offchain::is_validator() {
			// Create state hash.
			let state_hash: H256 = *state.root();
			match available_keys.get(0) {
				None => return Err("No active keys found"),
				Some(key) => {
					// Unwrap is okay here, we verified the data before.
					let auth_index = Self::calculate_signer_index(&authorities, key)
						.ok_or("Unable to calculate signer index")?;

					// Prepare summary
					let summary = SnapshotSummary {
						validator_set_id: <ValidatorSetId<T>>::get(),
						snapshot_id: next_nonce,
						state_hash,
						state_change_id: batch.stid,
						last_processed_blk: state_info.last_block.saturated_into(),
						withdrawals,
					};
					log::debug!(target:"ocex","Summary created by auth index: {:?}",auth_index);
					let signature = key.sign(&summary.encode()).ok_or("Private key not found")?;

					let body = serde_json::to_string(&ApprovedSnapshot {
						summary: summary.encode(),
						index: auth_index.saturated_into(),
						signature: signature.encode(),
					})
					.map_err(|_| "ApprovedSnapshot serialization failed")?;

					if let Err(err) = send_request(
						"submit_snapshot_api",
						&(AGGREGATOR.to_owned() + "/submit_snapshot"),
						body.as_str(),
					) {
						log::error!(target:"ocex","Error submitting signature: {:?}",err);
					}
					store_summary::<T>(summary, signature, auth_index.saturated_into()); // Casting is fine here
				},
			}
		}

		state_info.snapshot_id = batch.snapshot_id; // Store the processed nonce
		Self::store_state_info(state_info, &mut state)?;
		state.commit();
		store_trie_root(*state.root());
		log::info!(target:"ocex","updated trie root: {:?}", state.root());

		Ok(())
	}

	fn import_blk(
		blk: T::BlockNumber,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
		state_info: &mut StateInfo,
	) -> Result<(), &'static str> {
		log::debug!(target:"ocex","Importing block: {:?}",blk);

		if blk <= state_info.last_block.saturated_into() {
			return Err("BlockOutofSequence")
		}

		let messages = Self::ingress_messages(blk);

		for message in messages {
			// We don't care about any other message
			if let IngressMessages::Deposit(main, asset, amt) = message {
				add_balance(
					state,
					&Decode::decode(&mut &main.encode()[..])
						.map_err(|_| "account id decode error")?,
					asset,
					amt,
				)?
			}
		}

		state_info.last_block = blk.saturated_into();

		Ok(())
	}

	fn trades(
		trades: &Vec<Trade>,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	) -> Result<(), &'static str> {
		log::info!(target:"ocex","Settling trades...");
		for trade in trades {
			let config = Self::trading_pairs(trade.maker.pair.base, trade.maker.pair.quote)
				.ok_or("TradingPairNotFound")?;
			process_trade(state, trade, config)?
		}

		Ok(())
	}

	fn withdraw(
		request: &WithdrawalRequest<T::AccountId>,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
		stid: u64,
	) -> Result<Withdrawal<T::AccountId>, &'static str> {
		log::info!(target:"ocex","Settling withdraw request...");
		let amount = request.amount().map_err(|_| "decimal conversion error")?;
		let account_info = <Accounts<T>>::get(&request.main).ok_or("Main account not found")?;

		if !account_info.proxies.contains(&request.proxy) {
			// TODO: Check Race condition
			return Err("Proxy not found")
		}
		if !request.verify() {
			return Err("SignatureVerificationFailed")
		}
		sub_balance(
			state,
			&Decode::decode(&mut &request.main.encode()[..])
				.map_err(|_| "account id decode error")?,
			request.asset(),
			amount,
		)?;
		let withdrawal = request.convert(stid).map_err(|_| "Withdrawal conversion error")?;

		Ok(withdrawal)
	}

	fn process_batch(
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
		batch: &UserActionBatch<T::AccountId>,
		state_info: &mut StateInfo,
	) -> Result<Vec<Withdrawal<T::AccountId>>, &'static str> {
		if state_info.stid >= batch.stid {
			return Err("Invalid stid")
		}

		let mut withdrawals = Vec::new();
		// Process Ob messages
		for action in &batch.actions {
			match action {
				UserActions::Trade(trades) => Self::trades(trades, state)?,
				UserActions::Withdraw(request) => {
					let withdrawal = Self::withdraw(request, state, batch.stid)?;
					withdrawals.push(withdrawal);
				},
				UserActions::BlockImport(blk) =>
					Self::import_blk((*blk).saturated_into(), state, state_info)?,
				UserActions::Reset => {}, // Not for offchain worker
			}
		}

		Ok(withdrawals)
	}

	fn load_state_info(state: &TrieDBMut<LayoutV1<BlakeTwo256>>) -> StateInfo {
		match state.get(&STATE_INFO) {
			Ok(Some(data)) => StateInfo::decode(&mut &data[..]).unwrap_or_default(),
			Ok(None) => StateInfo::default(),
			Err(_) => StateInfo::default(),
		}
	}

	fn store_state_info(
		state_info: StateInfo,
		state: &mut TrieDBMut<LayoutV1<BlakeTwo256>>,
	) -> Result<(), &'static str> {
		let _ = state.insert(&STATE_INFO, &state_info.encode()).map_err(map_trie_error)?;
		Ok(())
	}

	fn calculate_signer_index(
		authorities: &[T::AuthorityId],
		expected_signer: &T::AuthorityId,
	) -> Option<usize> {
		let mut auth_index: Option<usize> = None;
		for (index, auth) in authorities.iter().enumerate() {
			if *expected_signer == *auth {
				auth_index = Some(index);
				break
			}
		}
		auth_index
	}
}

use parity_scale_codec::alloc::string::ToString;
use sp_std::borrow::ToOwned;

pub fn get_user_action_batch<T: Config>(id: u64) -> Option<UserActionBatch<T::AccountId>> {
	let body = serde_json::json!({ "id": id }).to_string();
	let result =
		match send_request("user_actions_batch", &(AGGREGATOR.to_owned() + "/snapshots"), &body) {
			Ok(encoded_batch) => encoded_batch,
			Err(err) => {
				log::error!(target:"ocex","Error fetching user actions batch for {:?}: {:?}",id,err);
				return None
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

pub fn load_signed_summary_and_send<T: Config>(snapshot_id: u64) {
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
					if let Err(err) = send_request(
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

pub fn store_summary<T: Config>(
	summary: SnapshotSummary<T::AccountId>,
	signature: <<T as Config>::AuthorityId as RuntimeAppPublic>::Signature,
	auth_index: u16,
) {
	let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
	key.append(&mut summary.snapshot_id.encode());
	let summay_ref = StorageValueRef::persistent(&key);
	summay_ref.set(&(summary, signature, auth_index));
}

pub fn send_request(log_target: &str, url: &str, body: &str) -> Result<Vec<u8>, &'static str> {
	let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(5_000));

	let body_len =
		serde_json::to_string(&body.as_bytes().len()).map_err(|_| "Unable to serialize")?;
	log::debug!(target:"ocex","Sending {} request with body len {}...",log_target,body_len);
	let request = http::Request::post(url, [body]);
	let pending: PendingRequest = request
		.add_header("Content-Type", "application/json")
		.add_header("Content-Length", body_len.as_str())
		.deadline(deadline)
		.send()
		.map_err(map_http_err)?;

	log::debug!(target:"ocex","Waiting for {} response...",log_target);
	let response: Response = pending
		.try_wait(deadline)
		.map_err(|_pending| "deadline reached")?
		.map_err(map_sp_runtime_http_err)?;

	if response.code != 200u16 {
		log::warn!(target:"ocex","Unexpected status code for {}: {:?}",log_target,response.code);
		return Err("request failed")
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

#[allow(clippy::boxed_local)]
pub fn map_trie_error<T, E>(err: Box<TrieError<T, E>>) -> &'static str {
	match *err {
		TrieError::InvalidStateRoot(_) => "Invalid State Root",
		TrieError::IncompleteDatabase(_) => "Incomplete Database",
		TrieError::ValueAtIncompleteKey(_, _) => "ValueAtIncompleteKey",
		TrieError::DecoderError(_, _) => "DecoderError",
		TrieError::InvalidHash(_, _) => "InvalidHash",
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
	jsonrpc: serde_json::Value,
	result: Vec<u8>,
	id: u64,
}

impl JSONRPCResponse {
	pub fn new(content: Vec<u8>) -> Self {
		Self { jsonrpc: "2.0".into(), result: content, id: 2 }
	}
}

use orderbook_primitives::types::ApprovedSnapshot;
