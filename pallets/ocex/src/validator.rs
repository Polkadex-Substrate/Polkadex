use crate::{
	pallet::ValidatorSetId,
	settlement::{add_balance, process_trade, sub_balance},
	snapshot::StateInfo,
	storage::store_trie_root,
	Config, Pallet, SnapshotNonce,
};
use orderbook_primitives::{
	types::{ApprovedSnapshot, Trade, UserActionBatch, UserActions, WithdrawalRequest},
	SnapshotSummary,
};
use parity_scale_codec::{Decode, Encode};
use polkadex_primitives::{ingress::IngressMessages, withdrawal::Withdrawal, AssetId};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{
	crypto::ByteArray,
	offchain::{Duration, HttpError},
	H256,
};
use sp_runtime::{
	offchain::{
		http,
		http::{Error, PendingRequest, Response},
		storage::StorageValueRef,
	},
	SaturatedConversion,
};
use sp_std::{boxed::Box, collections::btree_map::BTreeMap, vec::Vec};

use trie_db::{TrieError, TrieMut};

/// Key of the storage that stores the status of an offchain worker
pub const WORKER_STATUS: [u8; 28] = *b"offchain-ocex::worker_status";
const STATE_INFO: [u8; 25] = *b"offchain-ocex::state_info";
const LAST_PROCESSED_SNAPSHOT: [u8; 26] = *b"offchain-ocex::snapshot_id";
/// Aggregator endpoint: Even though it is centralized for now, it is trustless
/// as it verifies the signature and and relays them to destination.
/// As a future improvment, we can make it decentralized, by having the community run
/// such aggregation endpoints
pub const AGGREGATOR: &str = "https://ob.aggregator.polkadex.trade";

impl<T: Config> Pallet<T> {
	/// Runs the offchain worker computes the next batch of user actions and
	/// submits snapshot summary to aggregator endpoint
	pub fn run_on_chain_validation(_block_num: T::BlockNumber) -> Result<bool, &'static str> {
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
		if let None = Self::acquire_offchain_lock() {
			return Ok(false)
		}
		// Check the next batch to process
		let next_nonce = <SnapshotNonce<T>>::get().saturating_add(1);

		// Load the state to memory
		let mut root = crate::storage::load_trie_root();
		log::info!(target:"ocex","block: {:?}, state_root {:?}", _block_num, root);
		let mut storage = crate::storage::State;
		let mut state = OffchainState::load(&mut storage, &mut root);

		let mut state_info = match Self::load_state_info(&mut state) {
			Ok(info) => info,
			Err(err) => {
				log::error!(target:"ocex","Err loading state info from storage: {:?}",err);
				store_trie_root(H256::zero());
				return Err(err)
			},
		};

		let last_processed_nonce = state_info.snapshot_id;

		// Check if we already processed this snapshot and updated our offchain state.
		if last_processed_nonce == next_nonce {
			log::debug!(target:"ocex","Submitting last processed snapshot: {:?}",next_nonce);
			// resubmit the summary to aggregator
			load_signed_summary_and_send::<T>(next_nonce);
			return Ok(true)
		}

		log::info!(target:"ocex","last_processed_nonce: {:?}, next_nonce: {:?}",last_processed_nonce, next_nonce);

		if next_nonce.saturating_sub(last_processed_nonce) > 2 {
			if state_info.last_block == 0 {
				state_info.last_block = 4768083; // This is hard coded as the starting point
			}
			// We need to sync our off chain state
			for nonce in last_processed_nonce.saturating_add(1)..next_nonce {
				log::info!(target:"ocex","Syncing batch: {:?}",nonce);
				// Load the next ObMessages
				let batch = match get_user_action_batch::<T>(nonce) {
					None => {
						log::error!(target:"ocex","No user actions found for nonce: {:?}",nonce);
						return Ok(true)
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
				Self::store_state_info(state_info, &mut state);
				let root = state.commit()?;
				store_trie_root(root);
				log::debug!(target:"ocex","Stored state root: {:?}",root);
				return Ok(true)
			},
			Some(batch) => batch,
		};

		log::info!(target:"ocex","Processing user actions for nonce: {:?}",next_nonce);
		let withdrawals = Self::process_batch(&mut state, &batch, &mut state_info)?;

		// Create state hash and store it
		state_info.stid = batch.stid;
		state_info.snapshot_id = batch.snapshot_id; // Store the processed nonce
		Self::store_state_info(state_info.clone(), &mut state);
		let state_hash: H256 = state.commit()?;
		store_trie_root(state_hash);
		log::info!(target:"ocex","updated trie root: {:?}", state_hash);

		if sp_io::offchain::is_validator() {
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

		Ok(true)
	}

	fn import_blk(
		blk: T::BlockNumber,
		state: &mut OffchainState,
		state_info: &mut StateInfo,
	) -> Result<(), &'static str> {
		log::debug!(target:"ocex","Importing block: {:?}",blk);

		if blk != state_info.last_block.saturating_add(1).into() {
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

	fn trades(trades: &Vec<Trade>, state: &mut OffchainState) -> Result<(), &'static str> {
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
		state: &mut OffchainState,
		stid: u64,
	) -> Result<Withdrawal<T::AccountId>, &'static str> {
		log::info!(target:"ocex","Settling withdraw request...");
		let amount = request.amount().map_err(|_| "decimal conversion error")?;
		// FIXME: Don't remove these comments, will be reintroduced after fixing the race condition
		// let account_info = <Accounts<T>>::get(&request.main).ok_or("Main account not found")?;

		// if !account_info.proxies.contains(&request.proxy) {
		// 	// TODO: Check Race condition
		// 	return Err("Proxy not found")
		// }

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
		state: &mut OffchainState,
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
				UserActions::Withdraw(request,stid) => {
					let withdrawal = Self::withdraw(request, state, stid)?;
					withdrawals.push(withdrawal);
				},
				UserActions::BlockImport(blk) =>
					Self::import_blk((*blk).saturated_into(), state, state_info)?,
				UserActions::Reset => {}, // Not for offchain worker
			}
		}

		Ok(withdrawals)
	}

	pub(crate) fn load_state_info(state: &mut OffchainState) -> Result<StateInfo, &'static str> {
		match state.get(&STATE_INFO.to_vec())? {
			Some(data) => Ok(StateInfo::decode(&mut &data[..]).unwrap_or_default()),
			None => Ok(StateInfo::default()),
		}
	}

	fn store_state_info(state_info: StateInfo, state: &mut OffchainState) {
		state.insert(STATE_INFO.to_vec(), state_info.encode());
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

	pub(crate) fn get_offchain_balance(
		account: &polkadex_primitives::AccountId,
	) -> Result<BTreeMap<AssetId, Decimal>, &'static str> {
		let mut root = crate::storage::load_trie_root();
		let mut storage = crate::storage::State;
		let state = crate::storage::get_state_trie(&mut storage, &mut root);
		let balance: BTreeMap<AssetId, Decimal> =
			match state.get(account.as_slice()).map_err(crate::validator::map_trie_error)? {
				None => BTreeMap::new(),
				Some(encoded) => BTreeMap::decode(&mut &encoded[..])
					.map_err(|_| "Unable to decode balances for account")?,
			};
		Ok(balance)
	}

	pub(crate) fn get_state_info() -> Result<StateInfo, &'static str> {
		let mut root = crate::storage::load_trie_root();
		let mut storage = crate::storage::State;
		let mut state = OffchainState::load(&mut storage, &mut root);
		Self::load_state_info(&mut state)
	}
}

use crate::storage::OffchainState;
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

fn load_signed_summary_and_send<T: Config>(snapshot_id: u64) {
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

fn store_summary<T: Config>(
	summary: SnapshotSummary<T::AccountId>,
	signature: <<T as Config>::AuthorityId as RuntimeAppPublic>::Signature,
	auth_index: u16,
) {
	let mut key = LAST_PROCESSED_SNAPSHOT.to_vec();
	key.append(&mut summary.snapshot_id.encode());
	let summay_ref = StorageValueRef::persistent(&key);
	summay_ref.set(&(summary, signature, auth_index));
}

fn send_request(log_target: &str, url: &str, body: &str) -> Result<Vec<u8>, &'static str> {
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

/// Helper function to map trie error to a static str
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

/// Http Resposne body
#[derive(Serialize, Deserialize)]
pub struct JSONRPCResponse {
	jsonrpc: serde_json::Value,
	pub(crate) result: Vec<u8>,
	id: u64,
}

impl JSONRPCResponse {
	pub fn new(content: Vec<u8>) -> Self {
		Self { jsonrpc: "2.0".into(), result: content, id: 2 }
	}
}
