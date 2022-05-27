// Copyright (C) 2020-2022 Polkadex OU
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// This is file is modified from beefy-gadget from Parity Technologies (UK) Ltd.

use codec::Encode;
use curv::{arithmetic::Converter, elliptic::curves::Secp256k1, BigInt};
use frame_support::BoundedVec;
use futures::{FutureExt, StreamExt};
use log::*;
use multi_party_ecdsa::protocols::multi_party_ecdsa::gg_2020::{
	party_i::SignatureRecid,
	state_machine::{
		keygen::{Keygen, LocalKey, ProtocolMessage},
		sign::{OfflineProtocolMessage, OfflineStage, PartialSignature, SignManual},
		traits::RoundBlame,
	},
};
use round_based::{IsCritical, Msg, StateMachine};
use sc_client_api::{Backend, FinalityNotification, FinalityNotifications};
use serde::{Deserialize, Serialize};
use sp_api::{BlockId, ProvideRuntimeApi};
use std::{
	collections::{HashMap, VecDeque},
	fmt::Debug,
	marker::PhantomData,
	str::FromStr,
	sync::{mpsc::Sender, Arc, Mutex},
};

use sp_core::ecdsa::Signature;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::{Block, Header};

use thea_primitives::{
	crypto::Public,
	keygen::{
		KeygenRound, OfflineStageRound, ProvideSubProtocol, SigningSessionPayload, SubProtocol,
		TheaPayload,
	},
	payload::{Network, SignedTheaPayload, UnsignedTheaPayload},
	AuthorityId, TheaApi, ValidatorSet, ValidatorSetId, GENESIS_AUTHORITY_SET_ID,
};

use crate::{
	error::{self, Error},
	keystore::TheaKeystore,
	rounds::RoundTracker,
	utils::{
		convert_all_keygen_messages, convert_back_keygen_messages, convert_signature,
		find_authorities_change, generate_party_index_sequence, handle_error,
		handling_incoming_messages, threshold_parties, verify_payload,
	},
	Client,
};

/// Struct returning satate of round
/// Aggregates outputs from `keygen_status()` and `offline_status()`
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct RoundInfo {
	pub current_round: u16,
	pub offline_rounds: Vec<u16>,
	pub keygen_status: (u16, Vec<u16>),
	pub offline_statuses: Vec<(u16, Vec<u16>)>,
	pub signing_session_info: HashMap<u32, Vec<SigningSessionInfo>>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SigningSessionInfo {
	pub unsigned_payload: UnsignedTheaPayload,
	/* TODO: export inner `LocalSignature` from `SignManual`
	 *pub local_signature: LocalSignature, */
}

impl From<SigningSession> for SigningSessionInfo {
	fn from(ss: SigningSession) -> Self {
		Self { unsigned_payload: ss.payload }
	}
}

pub(crate) struct WorkerParams<C, BE, R> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub rpc_send: Arc<Mutex<Sender<RoundInfo>>>,
	pub key_store: Option<SyncCryptoStorePtr>,
}

/// A THEA worker plays  protocol
pub struct TheaWorker<B, C, BE, R>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
{
	_client: Arc<C>,
	#[allow(dead_code)]
	backend: Arc<BE>,
	runtime: Arc<R>,
	key_store: TheaKeystore,
	finality_notifications: FinalityNotifications<B>,
	rounds: Arc<RoundTracker>,
	rpc_send: Arc<Mutex<Sender<RoundInfo>>>,
	/// Note the Vec<SignManual> is indexed in the same order as unsigned payloads
	signing_sessions: HashMap<u32, Vec<SigningSession>>,
	blocks_containing_payloads: VecDeque<u32>,
	msg_counter: u64,
	_be: PhantomData<BE>,
}
//is_first_keygen_generation: bool,
#[derive(Clone)]
pub struct SigningSession {
	pub payload: UnsignedTheaPayload,
	pub sign_manual: SignManual,
}

impl<B, C, BE, R> TheaWorker<B, C, BE, R>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
{
	/// Return a new Thea worker instance.
	///
	/// Note that a Thea worker is only fully functional if a corresponding
	/// Thea pallet has been deployed on-chain.
	///
	/// The Thea pallet is needed in order to keep track of the Thea authority set.
	pub(crate) fn new(worker_params: WorkerParams<C, BE, R>) -> Self {
		let WorkerParams { client, backend, runtime, rpc_send, key_store } = worker_params;

		TheaWorker {
			_client: client.clone(),
			backend,
			runtime,
			key_store: key_store.into(),
			finality_notifications: client.finality_notification_stream(),
			rounds: Arc::new(RoundTracker::new()),
			rpc_send,
			signing_sessions: HashMap::new(),
			blocks_containing_payloads: VecDeque::new(),
			msg_counter: u64::MIN,
			_be: PhantomData::default(),
		}
	}
}

impl<B, C, BE, R> TheaWorker<B, C, BE, R>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
{
	/// Return the current active validator set at header `header`.
	///
	/// Note that the validator set could be `None`. This is the case if we don't find
	/// a THEA authority set change and we can't fetch the authority set from the
	/// THEA on-chain state.
	///
	/// Such a failure is usually an indication that the THEA pallet has not been deployed (yet).
	fn validator_set(&self, header: &B::Header) -> Option<ValidatorSet<AuthorityId>> {
		find_authorities_change::<B, AuthorityId>(header).or_else(|| {
			let at = BlockId::hash(header.hash());
			self.runtime.runtime_api().validator_set(&at).ok()
		})
	}

	/// Return true if New validators are going to be selected for Next Era
	fn is_validator_changed(&self, header: &B::Header) -> Option<bool> {
		let at = BlockId::hash(header.hash());
		self.runtime.runtime_api().is_validator_set_changed(&at).ok()
	}

	/// Return the next active validator set at header `header`.
	fn next_validator_set(&self, header: &B::Header) -> Option<ValidatorSet<AuthorityId>> {
		let at = BlockId::hash(header.hash());
		self.runtime.runtime_api().next_validator_set(&at).ok()
	}

	/// Refs current `signing_sessions` state
	pub fn get_current_signing_sessions(&self) -> &HashMap<u32, Vec<SigningSession>> {
		&self.signing_sessions
	}

	fn collect_unsigned_payloads(
		&self,
		block_id: &<<B as Block>::Header as Header>::Number,
	) -> Result<Vec<UnsignedTheaPayload>, Error> {
		let at = BlockId::Number(*block_id);
		let blk_num: u32 = u32::from_str(&block_id.to_string())?;
		Ok(self.runtime.runtime_api().unsigned_payloads_api(&at, blk_num)?)
	}

	fn collect_round_info(
		&self,
		validator_set_id: ValidatorSetId,
		// TODO: should this be for current block?
		block_id: &<<B as Block>::Header as Header>::Number,
	) -> Result<RoundInfo, Error> {
		let current_round = self.rounds.current_round(validator_set_id)?;
		let unsigned_payload: Vec<UnsignedTheaPayload> =
			self.collect_unsigned_payloads(block_id)?;
		let mut offline_rounds = vec![];
		let mut offline_statuses = vec![];
		for unsigned in unsigned_payload {
			offline_rounds
				.push(self.rounds.offline_current_round(validator_set_id, &unsigned.payload)?);
			offline_statuses.push(self.rounds.offline_status(validator_set_id, &unsigned.payload)?);
		}
		let keygen_status = self.rounds.keygen_status(validator_set_id)?;
		let signing_session_info = self
			.get_current_signing_sessions()
			.iter()
			.map(|(k, v)| (*k, v.clone().into_iter().map(SigningSessionInfo::from).collect()))
			.collect();
		Ok(RoundInfo {
			current_round,
			offline_rounds,
			keygen_status,
			offline_statuses,
			signing_session_info,
		})
	}

	// when keygen is finished or set id updated but no keygen happened, we need to update the round
	// tracker
	fn collect_keygen_payload(&mut self, id: u64) {
		if let Some(rounds) =
			self.get_mut_rounds_or_log("Failed to get mut ref to rounds for thea output pick up")
		{
			match rounds.pick_output(id) {
				Ok(local_key) => {
					if rounds.set_local_key(id, local_key).is_err() {
						error!(target: "thea", "Error setting thea's local key");
						return;
					}
					handle_error(rounds.set_keygen_completed(&id, true), "set_keygen_completed");
				},
				Err(err) => {
					error!(target: "thea", "Unable to get local key: {:?}",err);
					return;
				},
			}
			info!(target: "thea", "🎂 New key set complete");
		}
	}

	/// Note Thea protocol progresses only with each finalized block, so if finality fails then thea
	/// will not progress. We need to make sure every action taken by thea protocol must be only
	/// after the cause of that action is finalized in Polkadex.
	pub fn handle_finality_notification(&mut self, notification: FinalityNotification<B>) {
		debug!(target: "thea", "🥩 Got New Finality notification: {:?}", notification.header.number());
		let validator_changed = self.is_validator_changed(&notification.header).unwrap_or(false);
		debug!(target: "thea", "Validator changed? {}", validator_changed);
		if let Some(active) = if validator_changed {
			debug!(target: "thea", "Using next validator set");
			self.next_validator_set(&notification.header)
		} else {
			debug!(target: "thea", "Using current validator set");
			self.validator_set(&notification.header)
		} {
			if let Some(authority) = self.has_thea_key(&active) {
				let mut rng = (authority.0 as u64) + 100;
				// skip first keygen if one is started already
				if (active.id == GENESIS_AUTHORITY_SET_ID || validator_changed)
					&& self.rounds.get_keygen_not_started(&active.id)
				{
					debug!(target: "thea", "Thea Party index: {:?}", authority.0);
					debug!(target: "thea", "Thea AuthorityID: {:?}", authority.1);
					debug!(target: "thea", "🥩 New active validator set id: {:?}", active);
					if let Some(rounds) = self.get_mut_rounds_or_log(
						"FATAL: failed to obtain mutable rounds for thea keygen. terminating",
					) {
						rounds.new_round(active.id);
						debug!(target: "thea", "🥩 New Rounds for id: {:?}", active.id);
						if rounds
							.set_active_validator_len(&active.id, active.validators.len())
							.is_err()
						{
							log::error!(target:"thea", "unable to set active validator length: id: {:?}, len: {:?}",active.id,active.validators.len());
							return;
						}
						rounds.set_active_round_id(active.id);
						handle_error(
							rounds.set_keygen_not_started(&active.id, false),
							"set_keygen_not_started",
						);
						handle_error(
							rounds.set_validator_idx(&active.id, (authority.0 + 1) as u16),
							"set_validator_idx",
						);
						if let Err(err) = self.initialize_new_local_party(
							&active.id,
							active.validators.len() as u16,
							(authority.0 + 1) as u16,
						) {
							error!(target:"thea", "Unable to initialize new local party: id: {:?}, err: {:?}",active.id,err);
							return;
						}
						match self.start_keygen(active.id) {
							None => {
								error!(target: "thea", "Error in Keygen subp-protocol");
								return;
							},
							Some((messages, current_round)) => {
								info!(target: "thea", "Sending first key generation message");
								match self
									.generate_payload_and_submit::<ProtocolMessage, KeygenRound>(
										messages.as_ref(),
										current_round,
										&active,
										notification.hash,
										authority.clone(),
										&[0u8; 32], // no mater what's here
										rng,
									) {
									Ok(_) => {},
									Err(err) => {
										error!(target: "thea", "Error while submitting thea payload: {:?}",err);
										return;
									},
								}
							},
						}
					}
				} else if let Ok(is_finished) = self.rounds.is_finished(active.id) {
					if let Ok(is_keygen_completed) = self.rounds.get_keygen_completed(&active.id) {
						if !is_finished && !is_keygen_completed {
							rng += 1;
							if let Err(err) =
								self.progress_keygen_stage(&active, notification.hash, rng)
							{
								error!(target: "thea", "Error in progressing keygen stage: {:?}",err);
								return;
							}
							debug!(target: "thea", "Progressing for {:?}", &active);
						} else if is_finished && !is_keygen_completed {
							debug!(target: "thea", "Keygen Protocol Finished, pick the output");
							self.collect_keygen_payload(active.id);
							// cleaning up keygen payloads
							rng += 1;
							debug!(target: "thea", "Cleaning up after keygen");
							handle_error(
								self.generate_clean_keygen_onchain_submit(
									authority.0 as u16,
									authority.1.clone(),
									notification.hash,
									rng,
								),
								"generate_clean_keygen_onchain_submit",
							);
						}
					} else {
						error!(target:"thea","Unable to get keygen completed flag: id: {:?}",active.id);
						return;
					}
				}

				if let Ok(payloads) = self.rounds.get_offline_party(&active.id) {
					for op in payloads {
						debug!(target: "thea", "processing existing offline stage for: {:?}", &op);
						if let Ok(is_offline_finished) =
							self.rounds.is_offline_finished(active.id, &op)
						{
							if let Ok(is_offline_stage_completed) =
								self.rounds.get_offline_stage_completed(&active.id, &op)
							{
								if !is_offline_stage_completed && !is_offline_finished {
									let rng_increment = self.rng(&active.id, &op);
									if let Err(err) = self.progress_offline_stage(
										&active,
										notification.hash,
										&op,
										rng_increment,
									) {
										error!(target:"thea", "Error progressing offline stage: {:?}", err);
										return;
									}
								}
							} else {
								error!(target:"thea","Unable to get offline stage completed flag: id: {:?}, op: {:?}",active.id,op);
								return;
							}

							if let Ok(is_offline_stage_completed) =
								self.rounds.get_offline_stage_completed(&active.id, &op)
							{
								// Offline stage completed, pick the output
								if !is_offline_stage_completed && is_offline_finished {
									// TODO: Start from here
									debug!(target: "thea", "Offline Stage Completed, pick the output");
									if let Some(rounds) = Arc::get_mut(&mut self.rounds) {
										match rounds.pick_offline_output(active.id, &op) {
											Ok(completed_offline_stage) => {
												if rounds
													.set_completed_offline_stage(
														active.id,
														completed_offline_stage,
														&op,
													)
													.is_err()
												{
													error!(target: "thea", "Error setting thea's local key");
													return;
												}
												if rounds
													.set_offline_stage_completed(
														&active.id, true, &op,
													)
													.is_err()
												{
													error!(target:"thea","Unable to set offline stage completed to true: id: {:?} , payload: {:?}",active.id,op);
													return;
												}
												// now it's time to sign and clean up
												let rng_increment = self.rng(&active.id, &op);

												if let Ok(submission_blk) =
													self.rounds.submission_block(active.id, &op)
												{
													handle_error(
														self.start_signing_for_payload(
															notification.header.number(),
															&active,
															UnsignedTheaPayload {
																network: Network::ETHEREUM,
																payload: op,
																submission_blk,
															},
															rng_increment,
														),
														"start_signing_for_payload",
													);
												} else {
													error!(target:"thea","Unable to find submission_blk: {:?}",self
													.rounds
													.submission_block(active.id, &op) );
													return;
												}

												// TODO: implement cleanup per offline payload only
											},
											Err(err) => {
												error!(target: "thea", "Unable to pick offline stage output: {:?}",err);
												return;
											},
										}
									}
								}
							} else {
								error!(target:"thea","Unable to get offline stage completed flag: id: {:?}, op: {:?}",active.id,op);
							}
						}
					}
				}

				if let Ok(unsigned_payloads) =
					self.collect_unsigned_payloads(notification.header.number())
				{
					if unsigned_payloads.len() > 0 {
						debug!(target: "thea", "found {} unsigned payloads", unsigned_payloads.len());
						if let Ok(is_keygen_completed) =
							self.rounds.get_keygen_completed(&active.id)
						{
							for up in unsigned_payloads {
								if let Ok(is_offline_stage_started) =
									self.rounds.get_offline_stage_started(&active.id, &up.payload)
								{
									if is_keygen_completed && !is_offline_stage_started {
										match self.rounds.get_mpc_local_key(active.id) {
											Err(err) => {
												error!(target: "thea", "unable to get local key: {:?}",err);
												return;
											},
											Ok(local_key) => {
												debug!(target: "thea", "starting offline stage for: {:?}", &up.payload);
												let rng_increment =
													self.rng(&active.id, &up.payload);
												if let Err(err) = self.start_offline_stage(
													&active,
													local_key.clone(),
													notification.hash,
													authority.clone(),
													&(up.submission_blk, up.payload),
													rng_increment, // need to store block number
												) {
													error!(target:"thea","Unable to start new offline stage: id: {:?}, err: {:?}",active.id,err);
													return;
												}
											},
										}
									}
								} else {
									error!(target:"thea","unable to get offline stage started flag: id: {:?} payload: {:?}",active.id,up.payload);
									return;
								}
							}
						} else {
							error!(target:"thea","unable to get keygen stage completed flag: id: {:?}",active.id);
							return;
						}
					}
				}

				// Signing module is ready
				let _ = self.check_pending_signing_session(
					notification.header.number(),
					active.id,
					rng,
				);

				// rpc report section
				let report = match self.collect_round_info(active.id, notification.header.number())
				{
					Ok(data) => data,
					Err(e) => {
						debug!(target: "thea", "failed to collect round info for RPC. Details: {}", e.to_string());
						Default::default()
					},
				};
				match self.rpc_send.lock() {
					Ok(sender) => match sender.send(report) {
						Ok(_) => debug!(target: "thea", "RPC round info sent successfully"),
						Err(e) => {
							debug!(target: "thea", "failed to send round info into channel. Details: {}", e.to_string())
						},
					},
					Err(e) => {
						debug!(target: "thea", "failed to lock sender for RPC round info update. Details: {}", e.to_string())
					},
				}
			}
		}

		// resetting counter for the next block's rng
		self.msg_counter = u64::MIN;
	}

	// Start signing process for given payload
	// In parallel, sign payload and create a single broadcast message
	// Submit to runtime
	//fn check_new_payloads_for_signing(
	fn start_signing_for_payload(
		&mut self,
		current_block_number: &<<B as Block>::Header as Header>::Number,
		active: &ValidatorSet<AuthorityId>,
		payload: UnsignedTheaPayload,
		rng: u64,
	) -> Result<(), Error> {
		let at = BlockId::Number(*current_block_number);
		let blk_num: u32 = u32::from_str(&current_block_number.to_string())?;

		let authority_public_key =
			self.has_thea_key(active).ok_or(Error::UnableToFindAuthorityFromKeystore)?.1;
		let mut sessions = vec![];
		let mut signing_session_payload = SigningSessionPayload {
			partial_signatures: BoundedVec::default(),
			signer: Some(authority_public_key.clone()),
			set_id: active.id,
			auth_idx: self.rounds.get_validator_idx(&active.id)? - 1,
		};
		let cos = self.rounds.get_completed_offline_stage(active.id, &payload.payload)?;
		let (sign_manual, msg) = SignManual::new(BigInt::from_bytes(&payload.payload), cos)?;
		sessions.push(SigningSession { payload, sign_manual });
		signing_session_payload
			.partial_signatures
			.try_push(BoundedVec::try_from(serde_json::to_vec(&msg)?)?)?;
		if !sessions.is_empty() {
			let signature =
				self.key_store.sign(&authority_public_key, &signing_session_payload.encode())?;
			// Submit signed_payloads to runtime
			self.runtime.runtime_api().submit_signing_message(
				&at,
				blk_num,
				signing_session_payload,
				signature,
				rng,
			)??;
			self.blocks_containing_payloads.push_back(blk_num);
			self.signing_sessions.insert(blk_num, sessions);
		}

		Ok(())
	}

	// Check if there are any previous signing session to complete if so, complete the signature
	fn check_pending_signing_session(
		&mut self,
		current_block_number: &<<B as Block>::Header as Header>::Number,
		id: ValidatorSetId,
		rng: u64,
	) -> Result<(), Error> {
		let at = BlockId::Number(*current_block_number);
		let earliest_block_with_incomplete_sign_session =
			self.blocks_containing_payloads.get(0).ok_or(Error::NoPayloadPending)?;
		let others_partial_signatures = self
			.runtime
			.runtime_api()
			.signing_messages_api(&at, *earliest_block_with_incomplete_sign_session)?;

		// If we got less than the required number of signatures, then exit
		if others_partial_signatures.len()
			< threshold_parties(self.rounds.get_active_validator_len(&id)? as u16).into()
		{
			debug!(target: "thea", "Waiting for more partial signatures for payloads in blk: {:?} at finalized block: {:?}",earliest_block_with_incomplete_sign_session,current_block_number);
			return Ok(());
		}
		// others_partial_signatures is formatted as Vec<SigningSessionPayload>
		// SigningSessionPayload is contains a vector of PartialSignatures from a single party for
		// all payloads at a given block
		//
		// We need to reformat that to Vec<Vec<PartialSignature>>, where Vec<PartialSignature>
		// contains partial signs from all parties for a given payload
		let num_payloads: usize = others_partial_signatures[0].partial_signatures.len();
		let mut vec_signs: Vec<Vec<PartialSignature>> = vec![vec![]; num_payloads];
		let mut auth_idx = 0;
		for signing_session in others_partial_signatures {
			if auth_idx == self.rounds.get_validator_idx(&id)? {
				continue;
			}
			for (payload_idx, vec_signs_item) in
				vec_signs.iter_mut().enumerate().take(signing_session.partial_signatures.len())
			{
				let partial_sign: PartialSignature =
					serde_json::from_slice(&signing_session.partial_signatures[payload_idx])?;
				// vec_signs[payload_idx].push(partial_sign);
				vec_signs_item.push(partial_sign);
			}
			auth_idx += 1;
		}
		let sessions = self
			.signing_sessions
			.get(earliest_block_with_incomplete_sign_session)
			.ok_or(Error::UnableToFindSigningSession)?;

		let mut signed_payloads: Vec<SignedTheaPayload> = vec![];
		for session_idx in 0..sessions.len() {
			let session = sessions[session_idx].clone();
			let mpc_signature: SignatureRecid =
				session.sign_manual.complete(&vec_signs[session_idx])?.clone();
			let signature: Signature = convert_signature(&mpc_signature)?;
			// Create a SignedTheaPayload with the signature
			signed_payloads.push(SignedTheaPayload { payload: session.payload, signature });
		}
		let public_key = self.rounds.get_local_key(id)?;
		debug!(target: "thea", "verifying using local key: {:?}", &public_key);
		verify_payload(&public_key, &signed_payloads)?;
		// cleanup offline stage for this payload
		if let Some(tracker) = self.get_mut_rounds_or_log(
			"Failed to obtain mut ref to rounds tracker for signed payload OS cleanup",
		) {
			for sp in signed_payloads.iter() {
				tracker.remove_offline_stage(&id, &sp.payload.payload);
				tracker.remove_completed_stage(&id, &sp.payload.payload);
			}
		}
		if !signed_payloads.is_empty() {
			// Submit signed_payloads to runtime
			// as we might have more then one unsigned payload from prev blocks (network delays or
			// whatnot) - need to loop
			for payload in signed_payloads {
				self.runtime.runtime_api().submit_signed_payload(&at, payload, rng)??;
			}
			let blk = self.blocks_containing_payloads.pop_front().ok_or(Error::NoBlockInQueue)?;
			// self.blocks_containing_payloads.push_front(blk); // TODO: FOR DEBUG
			self.signing_sessions.remove(&blk);
			debug!(target: "thea", "Signing Completed for first unsigned payload in block: {:?}",blk);
		}
		Ok(())
	}

	fn progress_keygen_stage(
		&mut self,
		active: &ValidatorSet<AuthorityId>,
		at_hash: <B as Block>::Hash,
		rng: u64,
	) -> Result<(), Error> {
		let at = BlockId::hash(at_hash);
		let (auth_idx, authority_public_key) =
			self.has_thea_key(active).ok_or(Error::UnableToFindAuthorityFromKeystore)?;

		let (count, blames) = self.rounds.keygen_status(active.id)?;
		debug!(target: "thea", "Blames: {:?}, Count: {:?}",blames,count);
		let current_round = self.rounds.current_round(active.id)?;
		debug!(target: "thea", "Local party status: {:?}",current_round);
		for blame in blames {
			match self.runtime.runtime_api().keygen_messages_api(
				&at,
				blame - 1,
				current_round.into(),
			) {
				// Note round can never be Unknown, if it is Unknown then it's
				// considered Default value
				Ok(payload) if payload.round != KeygenRound::Unknown => {
					// if we fail here - which should not happen ever
					// we can still progress on next block when some unknown locks are released
					if let Some(rounds) = self.get_mut_rounds_or_log(
						"failed to acquire mut ref rounds for progress_keygen_stage",
					) {
						// debug!(target: "thea", "Read payload from sender: {:?},
						// payload: \n {:?}",blame-1,payload);
						let payload =
							convert_back_keygen_messages::<KeygenRound, ProtocolMessage>(payload)?;
						let (messages, current_round) =
							handling_incoming_messages::<ProtocolMessage, Keygen>(
								payload,
								rounds.mutable_local_party(active.id)?,
							)?;
						if !messages.is_empty() {
							self.generate_payload_and_submit::<ProtocolMessage, KeygenRound>(
								&messages,
								current_round,
								active,
								at_hash,
								(auth_idx, authority_public_key.clone()),
								&[0u8; 32],
								rng,
							)?;
						}
					}
				},
				_ => {
					debug!(target: "thea", "ignoring irrelevant payload for
					sender: {:?} from runtime at: {:?}",blame,at);
				},
			}
		}

		Ok(())
	}

	/// Once keygen is completed, we start the offline stage protocol
	/// that will set up the signing module.
	fn start_offline_stage(
		&mut self,
		active: &ValidatorSet<AuthorityId>,
		local_key: LocalKey<Secp256k1>,
		at_hash: <B as Block>::Hash,
		authority: (usize, Public),
		payload_details: &(u32, [u8; 32]),
		rng: u64,
	) -> Result<(), Error> {
		let mlp = self
			.get_mut_rounds_or_log("failed to acquire mut ref tracker for start_offline_stage")
			.map(|tracker| tracker.mutable_local_party(active.id));
		if let Some(Ok(local_party)) = mlp {
			match OfflineStage::new(
				local_party.party_ind(),
				generate_party_index_sequence(local_party.parties()),
				local_key,
			) {
				Err(err) => error!(target: "thea", "Error in initializing offline stage: {:?}",err),
				Ok(mut offline_stage) => {
					if offline_stage.wants_to_proceed() {
						match offline_stage.proceed() {
							Ok(_) => {
								debug!(target: "thea", "Offline stage for {:?} started and proceeded", payload_details);
							},
							Err(err) => {
								if err.is_critical() {
									error!(target:"thea", "Critical Error in OfflineStage StateMachine: {:?}", err);
								} else {
									warn!(target:"thea", "Error in OfflineStage StateMachine: {:?}", err);
								}
							},
						}
					}

					// WARN: needs additional review after changes if logic is not intact!
					self.generate_payload_and_submit::<OfflineProtocolMessage, OfflineStageRound>(
						&offline_stage.message_queue().to_owned(),
						offline_stage.current_round(),
						active,
						at_hash,
						authority,
						&payload_details.1,
						rng,
					)?;
					offline_stage.message_queue().clear();

					debug!(target: "thea", "Offline Stage Status: {:?}/7, RoundBlame: {:?}", offline_stage.current_round(),offline_stage.round_blame());
					if let Some(tracker) = self.get_mut_rounds_or_log("failed to acquire mut ref tracekir for set offline party and stage started") {
						tracker.set_offline_party(&active.id, offline_stage, payload_details.0, &payload_details.1)?;
						handle_error(tracker.set_offline_stage_started(&active.id, true, &payload_details.1), "set_offline_stage_started");
					}
				},
			}
		}
		Ok(())
	}

	fn progress_offline_stage(
		&mut self,
		active: &ValidatorSet<AuthorityId>,
		at_hash: <B as Block>::Hash,
		pd: &[u8; 32],
		rng: u64,
	) -> Result<(), Error> {
		let at = BlockId::hash(at_hash);
		let (auth_idx, authority_public_key) =
			self.has_thea_key(active).ok_or(Error::UnableToFindAuthorityFromKeystore)?;
		let (count, blames) = self.rounds.offline_status(active.id, pd)?;
		let current_round = self.rounds.offline_current_round(active.id, pd)?;
		debug!(target: "thea", "Offline Stage for {:?} => Blames: {:?}, Count: {:?}", &pd, blames, count);
		debug!(target: "thea", "Offline Stage for {:?} status: {:?}/7", &pd, current_round);

		for blame in blames {
			match self.runtime.runtime_api().offline_messages_api(
				&at,
				blame - 1,
				current_round.into(),
				pd,
			) {
				// Note round can never be Unknown, if it is Unknown then it's
				// considered Default value
				Ok(payload) if payload.round != OfflineStageRound::Unknown => {
					if let Some(tracker) = self.get_mut_rounds_or_log(
						"failed to acquire mut ref tracker for progress_offline_stage",
					) {
						// debug!(target: "thea", "Read payload from sender: {:?},
						// payload: \n {:?}",blame-1,payload);
						let payload = convert_back_keygen_messages::<
							OfflineStageRound,
							OfflineProtocolMessage,
						>(payload)?;
						let (messages, current_round) =
							handling_incoming_messages::<OfflineProtocolMessage, OfflineStage>(
								payload,
								tracker.mutable_offline_party(active.id, pd)?,
							)?;
						if !messages.is_empty() {
							self.generate_payload_and_submit::<OfflineProtocolMessage, OfflineStageRound>(
                                &messages,
                                current_round,
                                active,
                                at_hash,
								(auth_idx, authority_public_key.clone()),
								pd,
								rng,
                            )?;
						}
					}
				},
				_ => {
					debug!(target: "thea", "skipping irrelevant thea payload for
							sender: {:?} from runtime at: {:?}", blame, at);
				},
			}
		}

		Ok(())
	}

	// this method should be called each time it's output is used to properly increment it in the
	// cached store
	fn rng(&mut self, id: &u64, op: &[u8; 32]) -> u64 {
		let set_id = self.rounds.active_round_id();
		if let Some(tracker) = self.get_mut_rounds_or_log(
			"failed to obtain mut ref to round tracker for payload rng. using default value",
		) {
			let generated = tracker.rng_increment(id, op) + 105 + set_id + self.msg_counter; // 105 here to prevent clash
																				 // of new payload message
			debug!(target: "thea", "generated rng: {} for set: {}", generated, set_id);
			self.msg_counter += 1;
			generated
		} else {
			let def = 140 + set_id + self.msg_counter;
			debug!(target: "thea", "using default value {} for set {}", def, set_id);
			self.msg_counter += 1;
			def
		}
	}

	fn generate_clean_keygen_onchain_submit(
		&self,
		auth_idx: thea_primitives::AuthorityIndex,
		authority_public_key: Public,
		at_hash: <B as Block>::Hash,
		rng: u64,
	) -> Result<(), error::Error> {
		let at = BlockId::hash(at_hash);
		let signature = self.key_store.sign(&authority_public_key, &auth_idx.encode())?;
		match self.runtime.runtime_api().clean_keygen_messages(&at, auth_idx, signature, rng) {
			Ok(res) => match res {
				Ok(()) => {
					debug!(target: "thea", "successfully submitted the {:?} messages", auth_idx);
				},
				Err(err) => {
					error!(target: "thea", "Unable to sign transaction; {:?}", err);
				},
			},
			Err(err) => {
				error!(target: "thea", "Error in runtime api call: {:?}", err);
			},
		}
		Ok(())
	}

	#[allow(clippy::too_many_arguments)]
	fn generate_payload_and_submit<S, U>(
		&self,
		messages: &[Msg<S>],
		current_round: u16,
		active: &ValidatorSet<AuthorityId>,
		at_hash: <B as Block>::Hash,
		authority_details: (usize, Public),
		payload_array: &[u8; 32],
		rng: u64,
	) -> Result<(), error::Error>
	where
		S: Serialize,
		U: Debug + PartialEq + ProvideSubProtocol,
	{
		debug!(target: "thea", "generating payload for {:?}", &authority_details.1);
		let at = BlockId::hash(at_hash);
		if U::subprotocol() == SubProtocol::Keygen {
			let payload = TheaPayload {
				messages: convert_all_keygen_messages(messages)?,
				signer: Some(authority_details.1.clone()),
				set_id: active.id,
				auth_idx: authority_details.0 as u16,
				round: current_round.into(),
			};

			let signature = self.key_store.sign(&authority_details.1, &payload.encode())?;
			match self.runtime.runtime_api().submit_keygen_message(&at, payload, signature, rng) {
				Ok(res) => match res {
					Ok(()) => {
						debug!(target: "thea", "successfully submitted the {:?} messages for round {}", U::subprotocol(), current_round);
					},
					Err(err) => {
						error!(target: "thea", "Unable to sign transaction; {:?}", err);
					},
				},
				Err(err) => {
					error!(target: "thea", "Error in runtime api call: {:?}", err);
				},
			};
		} else {
			let payload = TheaPayload {
				messages: convert_all_keygen_messages(messages)?,
				signer: Some(authority_details.1.clone()),
				set_id: active.id,
				auth_idx: authority_details.0 as u16,
				round: current_round.into(),
			};

			let signature = self.key_store.sign(&authority_details.1, &payload.encode())?;
			match self.runtime.runtime_api().submit_offline_message(
				&at,
				payload,
				signature,
				rng,
				payload_array,
			) {
				Ok(res) => match res {
					Ok(()) => {
						debug!(target: "thea", "successfully submitted the {:?} messages", U::subprotocol());
					},
					Err(err) => {
						error!(target: "thea", "Unable to sign transaction; {:?}", err);
					},
				},
				Err(err) => {
					error!(target: "thea", "Error in runtime api call: {:?}", err);
				},
			};
		}
		Ok(())
	}

	/// Steps the internal state machine and blocks the current thread while doing so
	fn start_keygen(&mut self, id: ValidatorSetId) -> Option<(Vec<Msg<ProtocolMessage>>, u16)> {
		if let Some(tracker) = self.get_mut_rounds_or_log(&format!(
			"failed to get mutable ref to Arc'ed rounds for validator set:{} ",
			id
		)) {
			tracker.start_keygen(id)
		} else {
			None
		}
	}

	fn initialize_new_local_party(
		&mut self,
		id: &ValidatorSetId,
		n: u16,
		i: u16,
	) -> Result<(), Error> {
		// NOTE: index is convert from 0..n-1 to 1..n
		// ok() is okay since we expect the thea pallet to take care of corner cases
		let local_party = Keygen::new(i, threshold_parties(n), n).ok();
		if let Some(tracker) = self.get_mut_rounds_or_log(&format!(
			"failed to acquire mutable ref to round tracker for id: {}, n: {}, i: {}",
			id, n, i
		)) {
			tracker.set_local_party(id, local_party)
		} else {
			Err(Error::RoundNotFound(*id))
		}
	}

	/// Returns the authority key from keystore and index if available
	fn has_thea_key(
		&self,
		vaidator_set: &ValidatorSet<AuthorityId>,
	) -> Option<(usize, thea_primitives::crypto::Public)> {
		if let Some(authority_id) = self.key_store.authority_id(&vaidator_set.validators[..]) {
			vaidator_set
				.validators
				.iter()
				.position(|id| &authority_id == id)
				.map(|auth_idx| (auth_idx, authority_id))
		} else {
			None
		}
	}

	fn get_mut_rounds_or_log(&mut self, message: &str) -> Option<&mut RoundTracker> {
		match Arc::get_mut(&mut self.rounds) {
			Some(rounds) => Some(rounds),
			None => {
				error!(target: "thea", "{}", message);
				None
			},
		}
	}

	pub(crate) async fn run(&mut self) {
		loop {
			futures::select! {
				notification = self.finality_notifications.next().fuse() => {
					if let Some(notification) = notification {
						self.handle_finality_notification(notification);
					} else {
						return;
					}
				},
			}
		}
	}
}
