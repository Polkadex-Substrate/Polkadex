use chrono::Utc;
use std::{
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};

use bls_primitives::Public;
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use log::{debug, error, info, trace, warn};
use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{
		AccountAsset, AccountInfo, GossipMessage, ObMessage, StateSyncStatus, Trade, UserActions,
		WithdrawalRequest,
	},
	utils::{prepare_bitmap, return_set_bits, set_bit_field},
	ObApi, SnapshotSummary,
};
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
use polkadex_primitives::{
	ingress::IngressMessages, withdrawal::Withdrawal, AccountId, AssetId, BlockNumber,
};
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::Decimal;
use sc_client_api::{Backend, FinalityNotification};
use sc_network::PeerId;
use sc_network_gossip::{GossipEngine, Network as GossipNetwork};
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_consensus::SyncOracle;
use sp_core::{blake2_128, offchain::OffchainStorage};
use sp_runtime::{
	generic::BlockId,
	traits::{Block, Header, Zero},
};
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

use crate::{
	error::Error,
	gossip::{topic, GossipValidator},
	metric_add, metric_inc, metric_set,
	metrics::Metrics,
	snapshot::SnapshotStore,
	utils::*,
	Client, DbRef,
};
use orderbook_primitives::types::TradingPair;
use polkadex_primitives::ocex::TradingPairConfig;
use primitive_types::H128;

pub const ORDERBOOK_SNAPSHOT_SUMMARY_PREFIX: &[u8; 24] = b"OrderbookSnapshotSummary";
pub const ORDERBOOK_STATE_CHUNK_PREFIX: &[u8; 27] = b"OrderbookSnapshotStateChunk";

pub(crate) struct WorkerParams<B: Block, BE, C, SO, N, R> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub sync_oracle: SO,
	pub metrics: Option<Metrics>,
	pub is_validator: bool,
	pub message_sender_link: UnboundedReceiver<ObMessage>,
	/// Gossip network
	pub network: N,
	/// Chain specific Ob protocol name. See [`orderbook_protocol_name::standard_name`].
	pub protocol_name: sc_network::ProtocolName,
	pub _marker: PhantomData<B>,
	// last successful block snapshot created
	pub last_successful_block_number_snapshot_created: Arc<RwLock<BlockNumber>>,
	// memory db
	pub memory_db: DbRef,
	// working state root
	pub working_state_root: Arc<RwLock<[u8; 32]>>,
}

/// A Orderbook worker plays the Orderbook protocol
pub(crate) struct ObWorker<B: Block, BE, C, SO, N, R> {
	// utilities
	client: Arc<C>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	sync_oracle: SO,
	is_validator: bool,
	_network: Arc<N>,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	// Last processed state change id
	pub last_snapshot: Arc<RwLock<SnapshotSummary<AccountId>>>,
	// Working state root,
	pub working_state_root: Arc<RwLock<[u8; 32]>>,
	// Known state ids
	known_messages: BTreeMap<u64, ObMessage>,
	pending_withdrawals: Vec<Withdrawal<AccountId>>,
	/// Orderbook client metrics.
	metrics: Option<Metrics>,
	message_sender_link: UnboundedReceiver<ObMessage>,
	_marker: PhantomData<N>,
	// In memory store
	pub memory_db: DbRef,
	// Last finalized block
	last_finalized_block: BlockNumber,
	state_is_syncing: bool,
	// (snapshot id, chunk index) => status of sync
	sync_state_map: BTreeMap<u16, StateSyncStatus>,
	// last block at which snapshot was generated
	last_block_snapshot_generated: Arc<RwLock<BlockNumber>>,
	// latest stid
	latest_stid: Arc<RwLock<u64>>,
	// Map of trading pair configs
	trading_pair_configs: BTreeMap<TradingPair, TradingPairConfig>,
	orderbook_operator_public_key: Option<sp_core::ecdsa::Public>,
}

impl<B, BE, C, SO, N, R> ObWorker<B, BE, C, SO, N, R>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: ObApi<B>,
	SO: Send + Sync + Clone + 'static + SyncOracle,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
{
	/// Return a new BEEFY worker instance.
	///
	/// Note that a BEEFY worker is only fully functional if a corresponding
	/// BEEFY pallet has been deployed on-chain.
	///
	/// The BEEFY pallet is needed in order to keep track of the BEEFY authority set.
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, SO, N, R>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			sync_oracle,
			metrics,
			is_validator,
			message_sender_link,
			network,
			protocol_name,
			_marker,
			last_successful_block_number_snapshot_created: last_block_snapshot_generated,
			memory_db,
			working_state_root,
		} = worker_params;

		let last_snapshot = Arc::new(RwLock::new(SnapshotSummary::default()));
		let latest_stid = Arc::new(RwLock::new(0));
		let network = Arc::new(network);
		let gossip_validator = Arc::new(GossipValidator::new(latest_stid.clone()));
		let gossip_engine =
			GossipEngine::new(network.clone(), protocol_name, gossip_validator.clone(), None);

		ObWorker {
			client,
			backend,
			runtime,
			sync_oracle,
			is_validator,
			_network: network,
			gossip_engine,
			gossip_validator,
			memory_db,
			message_sender_link,
			state_is_syncing: false,
			metrics,
			last_snapshot,
			_marker: Default::default(),
			known_messages: Default::default(),
			working_state_root,
			pending_withdrawals: vec![],
			last_finalized_block: 0,
			sync_state_map: Default::default(),
			last_block_snapshot_generated,
			latest_stid,
			trading_pair_configs: Default::default(),
			orderbook_operator_public_key: None,
		}
	}

	/// The function checks whether a snapshot of the blockchain should be generated based on the
	/// pending withdrawals and block interval and last stid
	///
	/// # Parameters
	/// - &self: a reference to an instance of a struct implementing some trait
	/// # Returns
	/// - bool: a boolean indicating whether a snapshot should be generated
	pub fn should_generate_snapshot(&self) -> bool {
		let at = BlockId::Number(self.last_finalized_block.saturated_into());
		// Get the snapshot generation intervals from the runtime API for the last finalized block
		let (pending_withdrawals_interval, block_interval) = self
			.runtime
			.runtime_api()
			.get_snapshot_generation_intervals(&at)
			.expect("Expecting snapshot generation interval api to be available");

		let last_accepted_stid: u64 = self
			.runtime
			.runtime_api()
			.get_last_accepted_stid(&BlockId::Number(self.client.info().best_number))
			.expect("Expecting OCEX APIs to be available");

		// Check if a snapshot should be generated based on the pending withdrawals interval and
		// block interval
		if (pending_withdrawals_interval <= self.pending_withdrawals.len() as u64 ||
			block_interval <
				self.last_finalized_block
					.saturating_sub(*self.last_block_snapshot_generated.read())) &&
			last_accepted_stid < *self.latest_stid.read()
		// there is something new after last snapshot
		{
			info!(target:"orderbook", "Snapshot should be generated");
			return true
		}
		// If a snapshot should not be generated, return false
		false
	}

	pub fn process_withdraw(
		&mut self,
		withdraw: WithdrawalRequest,
		stid: u64,
	) -> Result<(), Error> {
		info!("Processing withdrawal request: {:?}", withdraw);
		let mut memory_db = self.memory_db.write();
		let mut working_state_root = self.working_state_root.write();
		let mut trie = Self::get_trie(&mut memory_db, &mut working_state_root);

		// Get main account
		let proxies = trie.get(&withdraw.main.encode())?.ok_or(Error::MainAccountNotFound)?;

		let account_info = AccountInfo::decode(&mut &proxies[..])?;
		// Check proxy registration
		if !account_info.proxies.contains(&withdraw.proxy) {
			return Err(Error::ProxyNotAssociatedWithMain)
		}
		// Verify signature
		if !withdraw.verify() {
			return Err(Error::WithdrawSignatureCheckFailed)
		}
		// Deduct balance
		sub_balance(&mut trie, withdraw.account_asset(), withdraw.amount()?)?;
		// Commit the trie
		trie.commit();
		drop(trie);
		drop(memory_db);
		drop(working_state_root);
		// Queue withdrawal
		self.pending_withdrawals.push(withdraw.try_into()?);
		info!(target:"orderbook","Queued withdrawal to pending list");
		// Check if snapshot should be generated or not
		if self.should_generate_snapshot() {
			if let Err(err) = self.snapshot(stid) {
				log::error!(target:"orderbook", "Couldn't generate snapshot after reaching max pending withdrawals: {:?}",err);
				*self.last_block_snapshot_generated.write() = self.last_finalized_block;
			}
		}
		Ok(())
	}

	pub fn handle_blk_import(&mut self, num: BlockNumber) -> Result<(), Error> {
		info!("Handling block import: {:?}", num);
		let mut memory_db = self.memory_db.write();
		let mut working_state_root = self.working_state_root.write();
		let mut trie = Self::get_trie(&mut memory_db, &mut working_state_root);

		// Get the ingress messsages for this block
		let messages = self
			.runtime
			.runtime_api()
			.ingress_messages(&BlockId::number(num.saturated_into()))
			.expect("Expecting ingress messages api to be available");

		{
			// 3. Execute RegisterMain, AddProxy, RemoveProxy, Deposit messages
			for message in messages {
				match message {
					IngressMessages::RegisterUser(main, proxy) =>
						register_main(&mut trie, main, proxy)?,
					IngressMessages::Deposit(main, asset, amt) =>
						deposit(&mut trie, main, asset, amt)?,
					IngressMessages::AddProxy(main, proxy) => add_proxy(&mut trie, main, proxy)?,
					IngressMessages::RemoveProxy(main, proxy) =>
						remove_proxy(&mut trie, main, proxy)?,
					_ => {},
				}
			}
			// Commit the trie
			trie.commit();
		}
		Ok(())
	}

	pub fn get_validator_key(&self, active_set: &Vec<AuthorityId>) -> Result<Public, Error> {
		let available_bls_keys: Vec<Public> = bls_primitives::crypto::bls_ext::all();
		info!(target:"orderbook","📒 Avaialble BLS keys: {:?}",available_bls_keys);
		info!(target:"orderbook","📒 Active BLS keys: {:?}",active_set);
		// Get the first available key in the validator set.
		let mut validator_key = None;
		for key in available_bls_keys {
			if active_set.contains(&orderbook_primitives::crypto::AuthorityId::from(key)) {
				validator_key = Some(key);
				break
			}
		}
		if validator_key.is_none() {
			info!(target:"orderbook","📒 No validator key found for snapshotting. Skipping snapshot signing.");
			return Err(Error::Keystore(
				"No validator key found for snapshotting. Skipping snapshot signing.".into(),
			))
		}
		Ok(validator_key.unwrap())
	}

	pub fn snapshot(&mut self, stid: u64) -> Result<(), Error> {
		info!(target:"orderbook","📒 Generating snapshot");
		let at = BlockId::number(self.last_finalized_block.saturated_into());
		let next_snapshot_id = self
			.runtime
			.runtime_api()
			.get_latest_snapshot(&at)
			.expect("Expected get_latest_snapshot to be available")
			.snapshot_id
			.saturating_add(1);

		let mut summary = self.store_snapshot(stid, next_snapshot_id)?;
		if !self.is_validator {
			info!(target:"orderbook","📒 Not a validator, skipping snapshot signing.");
			// We are done if we are not a validator
			return Ok(())
		}
		let active_set = self.runtime.runtime_api().validator_set(&at)?.validators;
		let signing_key = self.get_validator_key(&active_set)?;
		info!(target:"orderbook","Signing snapshot with: {:?}",hex::encode(signing_key.0));
		let initial_summary = summary.sign_data();
		let signature = match bls_primitives::crypto::sign(&signing_key, &summary.sign_data()) {
			Some(sig) => sig,
			None => {
				error!(target:"orderbook","📒 Failed to sign snapshot, not able to sign with validator key.");
				return Err(Error::SnapshotSigningFailed)
			},
		};

		summary.aggregate_signature = Some(signature);
		let bit_index = active_set.iter().position(|v| v == &signing_key.into()).unwrap();
		info!(target:"orderbook","📒 Signing snapshot with bit index: {:?}",bit_index);
		set_bit_field(&mut summary.bitflags, bit_index as u16);
		assert_eq!(initial_summary, summary.sign_data());
		assert!(bls_primitives::crypto::bls_ext::verify(
			&signing_key,
			&initial_summary,
			&signature,
		));
		// send it to runtime
		if self
			.runtime
			.runtime_api()
			.submit_snapshot(&BlockId::number(self.last_finalized_block.into()), summary)
			.expect("Something went wrong with the submit_snapshot runtime api; qed.")
			.is_err()
		{
			error!(target:"orderbook","📒 Failed to submit snapshot to runtime");
			return Err(Error::FailedToSubmitSnapshotToRuntime)
		}
		Ok(())
	}

	pub fn handle_action(&mut self, action: &ObMessage) -> Result<(), Error> {
		info!(target:"orderbook","📒 Processing action: {:?}", action);
		match action.action.clone() {
			// Get Trie here itself and pass to required function
			// No need to change Test cases
			UserActions::Trade(trades) => {
				let mut memory_db = self.memory_db.write();
				let mut working_state_root = self.working_state_root.write();
				let mut trie = Self::get_trie(&mut memory_db, &mut working_state_root);

				for trade in trades {
					let config = self
						.trading_pair_configs
						.get(&trade.maker.pair)
						.ok_or(Error::TradingPairConfigNotFound)?
						.clone();
					process_trade(&mut trie, trade, config)?
				}
				// Commit the trie
				trie.commit();
			},
			UserActions::Withdraw(withdraw) => self.process_withdraw(withdraw, action.stid)?,
			UserActions::BlockImport(num) => self.handle_blk_import(num)?,
			UserActions::Snapshot => self.snapshot(action.stid)?,
		}
		*self.latest_stid.write() = action.stid;
		// Multicast the message to other peers
		let gossip_message = GossipMessage::ObMessage(Box::new(action.clone()));
		self.gossip_engine.gossip_message(topic::<B>(), gossip_message.encode(), true);
		info!(target:"orderbook","Message with stid: {:?} gossiped to others",self.latest_stid.read());
		Ok(())
	}

	// Checks if we need to sync the orderbook state before processing the messages.
	pub async fn check_state_sync(&mut self) -> Result<(), Error> {
		info!(target:"orderbook","📒 Checking state sync");
		// X->Y sync: Ask peers to send the missed stid
		if !self.known_messages.is_empty() {
			info!(target:"orderbook","📒 Known messages: {:?}", self.known_messages);
			// Collect all known stids
			let mut known_stids = self.known_messages.keys().collect::<Vec<&u64>>();
			known_stids.sort_unstable(); // unstable is fine since we know stids are unique
							 // if the next best known stid is not available then ask others
			if *known_stids[0] != self.last_snapshot.read().state_change_id.saturating_add(1) {
				// Ask other peers to send us the requests stids.
				info!(target:"orderbook","📒 Asking peers to send us the missed \
                stids: last processed stid: {:?}, best known stid: {:?} ",
                    self.last_snapshot.read().state_change_id, known_stids[0]);
				let message = GossipMessage::WantStid(
					self.last_snapshot.read().state_change_id,
					*known_stids[0],
				);
				let mut peers = self
					.gossip_validator
					.peers
					.read()
					.clone()
					.iter()
					.cloned()
					.collect::<Vec<PeerId>>();
				let mut fullnodes = self
					.gossip_validator
					.fullnodes
					.read()
					.clone()
					.iter()
					.cloned()
					.collect::<Vec<PeerId>>();
				peers.append(&mut fullnodes);
				// TODO: Should we even send it out to everyone we know?
				self.gossip_engine.send_message(peers, message.encode());
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			} else {
				info!(target: "orderbook", "📒 sync request not required, we know the next stid");
			}
		} else {
			info!(target: "orderbook", "📒 No new messages known after stid: {:?}",self.last_snapshot.read().state_change_id);
		}
		Ok(())
	}

	pub fn load_state_from_data(
		&mut self,
		data: &[u8],
		summary: &SnapshotSummary<AccountId>,
	) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Loading state from snapshot data ({} bytes)", data.len());
		match serde_json::from_slice::<SnapshotStore>(data) {
			Ok(store) => {
				info!(target: "orderbook", "📒 Loaded state from snapshot data ({} bytes)",  store.map.len());
				let memory_db_write_lock = self.memory_db.write();
				let mut memory_db = memory_db_write_lock.clone();
				memory_db.load_from(store.map);
				let summary_clone = summary.clone();
				*self.last_snapshot.write() = summary_clone;
			},
			Err(err) => {
				error!(target: "orderbook", "📒 Error decoding snapshot data: {:?}", err);
				return Err(Error::Backend(format!("Error decoding snapshot data: {:?}", err)))
			},
		}
		Ok(())
	}

	pub async fn process_new_user_action(&mut self, action: &ObMessage) -> Result<(), Error> {
		// Check if stid is newer or not
		if action.stid <= *self.latest_stid.read() {
			// Ignore stids we already know.
			warn!(target:"orderbook","Ignoring old message: given: {:?}, latest stid: {:?}",action.stid,self.latest_stid.read());
			return Ok(())
		}
		info!(target: "orderbook", "📒 Processing new user action: {:?}", action);
		if let Some(expected_singer) = self.orderbook_operator_public_key {
			if !action.verify(&expected_singer) {
				error!(target: "orderbook", "📒 Invalid signature for action: {:?}",action);
				return Err(Error::SignatureVerificationFailed)
			}
		} else {
			warn!(target: "orderbook", "📒 Orderbook operator public key not set");
			return Err(Error::SignatureVerificationFailed)
		}
		info!(target: "orderbook", "📒 Ob message recieved stid: {:?}",action.stid);
		// Cache the message
		self.known_messages.insert(action.stid, action.clone());
		if self.sync_oracle.is_major_syncing() | self.state_is_syncing {
			info!(target: "orderbook", "📒 Ob message cached for sync to complete: stid: {:?}",action.stid);
			return Ok(())
		}
		self.check_state_sync().await?;
		self.check_stid_gap_fill().await?;
		Ok(())
	}

	#[cfg(test)]
	pub fn get_offline_storage(&mut self, id: u64) -> Option<Vec<u8>> {
		let offchain_storage = self.backend.offchain_storage().unwrap();
		let result = offchain_storage.get(ORDERBOOK_SNAPSHOT_SUMMARY_PREFIX, &id.encode());
		return result
	}

	pub fn store_snapshot(
		&mut self,
		state_change_id: u64,
		snapshot_id: u64,
	) -> Result<SnapshotSummary<AccountId>, Error> {
		info!(target: "orderbook", "📒 Storing snapshot: {:?}", snapshot_id);
		if let Some(mut offchain_storage) = self.backend.offchain_storage() {
			let store = SnapshotStore { map: self.memory_db.read().data().clone() };
			return match serde_json::to_vec(&store) {
				Ok(data) => {
					info!(target: "orderbook", "📒 Stored snapshot data ({} bytes)", data.len());
					let mut state_chunk_hashes = vec![];
					// Slice the data into chunks of 10 MB
					let mut chunks = data.chunks(10 * 1024 * 1024);
					for chunk in &mut chunks {
						let chunk_hash = H128::from(blake2_128(chunk));
						offchain_storage.set(
							ORDERBOOK_STATE_CHUNK_PREFIX,
							chunk_hash.0.as_ref(),
							chunk,
						);
						state_chunk_hashes.push(chunk_hash);
					}

					let withdrawals = self.pending_withdrawals.clone();
					self.pending_withdrawals.clear();
					info!(target: "orderbook", "📒 Stored snapshot withdrawals ({} bytes)", withdrawals.len());

					let working_state_root_read_lock = self.working_state_root.read();
					let working_state_root = *working_state_root_read_lock;

					let summary = SnapshotSummary {
						snapshot_id,
						state_root: working_state_root.into(),
						state_change_id,
						bitflags: vec![],
						withdrawals,
						aggregate_signature: None,
						state_chunk_hashes,
					};

					offchain_storage.set(
						ORDERBOOK_SNAPSHOT_SUMMARY_PREFIX,
						&snapshot_id.encode(),
						&summary.encode(),
					);
					Ok(summary)
				},
				Err(err) => Err(Error::Backend(format!("Error serializing the data: {:?}", err))),
			}
		}
		Err(Error::Backend("Offchain Storage not Found".parse().unwrap()))
	}

	pub fn load_snapshot(&mut self, summary: &SnapshotSummary<AccountId>) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Loading snapshot: {:?}", summary.snapshot_id);
		if summary.snapshot_id == 0 {
			// Nothing to do if we are on state_id 0
			return Ok(())
		}
		if let Some(offchain_storage) = self.backend.offchain_storage() {
			let mut data = Vec::new();
			for chunk_hash in &summary.state_chunk_hashes {
				if let Some(mut chunk) =
					offchain_storage.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk_hash.0.as_ref())
				{
					let computed_hash = H128::from(blake2_128(&chunk));
					if computed_hash != *chunk_hash {
						warn!(target:"orderbook","📒 orderbook state hash mismatch: computed: {:?}, expected: {:?}",computed_hash,chunk_hash);
						return Err(Error::StateHashMisMatch)
					}
					data.append(&mut chunk);
				}
			}
			self.load_state_from_data(&data, summary)?;
		} else {
			warn!(target:"orderbook","📒 orderbook state chunk not found");
		}
		Ok(())
	}

	// Checks if we have all stids to drive the state and then drive it.
	pub async fn check_stid_gap_fill(&mut self) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Checking for stid gap fill");
		let mut last_snapshot = self.last_snapshot.read().state_change_id.saturating_add(1);

		while let Some(action) = self.known_messages.remove(&last_snapshot) {
			if let Err(err) = self.handle_action(&action) {
				match err {
					Error::Keystore(_) =>
						error!(target:"orderbook","📒 BLS session key not found: {:?}",err),
					_ => {
						error!(target:"orderbook","📒 Error processing action: {:?}",err);
						// The node found an error during processing of the action. This means we
						// need to snapshot and drop everything else
						self.snapshot(action.stid)?;
						// We forget about everything else from cache.
						self.known_messages.clear();
						break
					},
				}
			}
			metric_set!(self, ob_state_id, last_snapshot);
			last_snapshot = last_snapshot.saturating_add(1);
		}
		// We need to sub 1 since that last processed is one stid less than the not available
		// when while loop is broken
		info!(target:"orderbook","Setting last snapshot's stid to : {:?}",last_snapshot.saturating_sub(1));
		self.last_snapshot.write().state_change_id = last_snapshot.saturating_sub(1);
		Ok(())
	}

	pub fn want_stid(&mut self, from: &u64, to: &u64, peer: Option<PeerId>) {
		info!(target: "orderbook", "📒 Want stid: {:?} - {:?}", from, to);
		if let Some(peer) = peer {
			info!(target: "orderbook", "📒 Sending stid request to peer: {:?}", peer);
			let mut messages = vec![];
			for stid in *from..=*to {
				// We dont allow gossip messsages to be greater than 10MB
				if messages.encoded_size() >= 10 * 1024 * 1024 {
					// If we reach size limit, we send data in chunks of 10MB.
					info!(target: "orderbook", "📒 Sending stid chunk: {:?}", messages.len());
					let message = GossipMessage::Stid(Box::new(messages));
					self.gossip_engine.send_message(vec![peer], message.encode());
					metric_inc!(self, ob_messages_sent);
					metric_add!(self, ob_data_sent, message.encoded_size() as u64);
					messages = vec![] // Reset the buffer
				}
				if let Some(msg) = self.known_messages.get(&stid) {
					info!(target: "orderbook", "📒 known_messages: {:?}", self.known_messages.len());
					messages.push(msg.clone());
				}
			}
			// Send the final chunk if any
			if !messages.is_empty() {
				let message = GossipMessage::Stid(Box::new(messages));
				self.gossip_engine.send_message(vec![peer], message.encode());
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			} else {
				info!(target: "orderbook", "📒 No stids to send to peer: {:?}", peer)
			}
		}
	}

	pub async fn got_stids_via_gossip(&mut self, messages: &Vec<ObMessage>) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Got stids via gossip: {:?}", messages.len());
		for message in messages {
			// TODO: handle reputation change.
			self.known_messages.entry(message.stid).or_insert(message.clone());
		}
		self.check_stid_gap_fill().await
	}

	// Expects the set bits in the bitmap to be missing chunks
	pub async fn want(&mut self, snapshot_id: &u64, bitmap: &[u128], remote: Option<PeerId>) {
		info!(target: "orderbook", "📒 Want snapshot: {:?} - {:?}", snapshot_id, bitmap);
		// Only respond if we are a fullnode
		// TODO: Should we respond if we are also syncing???
		if !self.is_validator {
			if let Some(peer) = remote {
				let mut chunks_we_have = vec![];
				let at = BlockId::Number(self.last_finalized_block.saturated_into());
				if let Ok(Some(summary)) =
					self.runtime.runtime_api().get_snapshot_by_id(&at, *snapshot_id)
				{
					if let Some(offchain_storage) = self.backend.offchain_storage() {
						let required_indexes: Vec<u16> = return_set_bits(bitmap);
						for chunk_index in required_indexes {
							if offchain_storage
								.get(
									ORDERBOOK_STATE_CHUNK_PREFIX,
									summary.state_chunk_hashes[chunk_index as usize]
										.encode()
										.as_ref(),
								)
								.is_some()
							{
								chunks_we_have.push(chunk_index);
							}
						}
					}
				} else {
					// TODO: Reduce reputation if else block is happens
				}

				if !chunks_we_have.is_empty() {
					let message = GossipMessage::Have(*snapshot_id, prepare_bitmap(chunks_we_have));
					self.gossip_engine.send_message(vec![peer], message.encode());
					metric_inc!(self, ob_messages_sent);
					metric_add!(self, ob_data_sent, message.encoded_size() as u64);
				}
			}
		}
	}

	pub async fn have(&mut self, snapshot_id: &u64, bitmap: &[u128], remote: Option<PeerId>) {
		info!(target: "orderbook", "📒 Have snapshot: {:?} - {:?}", snapshot_id, bitmap);
		if let Some(peer) = remote {
			// Note: Set bits here are available for syncing
			let available_chunks: Vec<u16> = return_set_bits(bitmap);
			let mut want_chunks = vec![];
			for index in available_chunks {
				if let Some(chunk_status) = self.sync_state_map.get_mut(&index) {
					if *chunk_status == StateSyncStatus::Unavailable {
						want_chunks.push(index);
						*chunk_status = StateSyncStatus::InProgress(peer, Utc::now().timestamp());
					}
				}
			}
			if !want_chunks.is_empty() {
				let message =
					GossipMessage::RequestChunk(*snapshot_id, prepare_bitmap(want_chunks));
				self.gossip_engine.send_message(vec![peer], message.encode());
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			}
		}
	}

	pub async fn request_chunk(
		&mut self,
		snapshot_id: &u64,
		bitmap: &[u128],
		remote: Option<PeerId>,
	) {
		info!(target: "orderbook", "📒 Request chunk: {:?} - {:?}", snapshot_id, bitmap);
		if let Some(peer) = remote {
			if let Some(offchian_storage) = self.backend.offchain_storage() {
				let at = BlockId::Number(self.last_finalized_block.saturated_into());
				if let Ok(Some(summary)) =
					self.runtime.runtime_api().get_snapshot_by_id(&at, *snapshot_id)
				{
					let chunk_indexes: Vec<u16> = return_set_bits(bitmap);
					for index in chunk_indexes {
						match summary.state_chunk_hashes.get(index as usize) {
							None => {
								log::warn!(target:"orderbook","Chunk hash not found for index: {:?}",index)
							},
							Some(chunk_hash) => {
								if let Some(data) = offchian_storage
									.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk_hash.0.as_ref())
								{
									let message = GossipMessage::Chunk(*snapshot_id, index, data);
									self.gossip_engine.send_message(vec![peer], message.encode());
									metric_inc!(self, ob_messages_sent);
									metric_add!(self, ob_data_sent, message.encoded_size() as u64);
								} else {
									warn!(target:"orderbook","No chunk found for index: {:?}",index)
								}
							},
						}
					}
				} else {
					warn!(target:"orderbook","No snapshot found for request chunk")
				}
			} else {
				warn!(target:"orderbook","No offchain storage found for request chunk")
			}
		} else {
			warn!(target:"orderbook","No peer found for request chunk")
		}
	}

	pub fn process_chunk(&mut self, snapshot_id: &u64, index: &u16, data: &[u8]) {
		info!(target: "orderbook", "📒 Chunk snapshot: {:?} - {:?} - {:?}", snapshot_id, index, data.len());
		if let Some(mut offchian_storage) = self.backend.offchain_storage() {
			let at = BlockId::Number(self.last_finalized_block.saturated_into());
			if let Ok(Some(summary)) =
				self.runtime.runtime_api().get_snapshot_by_id(&at, *snapshot_id)
			{
				match summary.state_chunk_hashes.get(*index as usize) {
					None =>
						warn!(target:"orderbook","Invalid index recvd, index > length of state chunk hashes"),
					Some(expected_hash) => {
						let computed_hash: H128 = H128::from(blake2_128(data));
						if *expected_hash == computed_hash {
							// Store the data
							offchian_storage.set(
								ORDERBOOK_STATE_CHUNK_PREFIX,
								expected_hash.0.as_ref(),
								data,
							);
							// Update sync status map
							self.sync_state_map
								.entry(*index)
								.and_modify(|status| {
									*status = StateSyncStatus::Available;
								})
								.or_insert(StateSyncStatus::Available);
						}
					},
				}
			}
		}
	}

	#[cfg(test)]
	pub fn get_sync_state_map_value(&self, key: u16) -> StateSyncStatus {
		self.sync_state_map.get(&key).unwrap().clone()
	}

	pub async fn process_gossip_message(
		&mut self,
		message: &GossipMessage,
		remote: Option<PeerId>,
	) -> Result<(), Error> {
		info!(target:"orderbook","Processing gossip message: {:?}",message);
		metric_inc!(self, ob_messages_recv);
		metric_add!(self, ob_data_recv, message.encoded_size() as u64);
		match message {
			GossipMessage::WantStid(from, to) => self.want_stid(from, to, remote),
			GossipMessage::Stid(messages) => self.got_stids_via_gossip(messages).await?,
			GossipMessage::ObMessage(msg) => self.process_new_user_action(msg).await?,
			GossipMessage::Want(snap_id, bitmap) => self.want(snap_id, bitmap, remote).await,
			GossipMessage::Have(snap_id, bitmap) => self.have(snap_id, bitmap, remote).await,
			GossipMessage::RequestChunk(snap_id, bitmap) =>
				self.request_chunk(snap_id, bitmap, remote).await,
			GossipMessage::Chunk(snap_id, index, data) => self.process_chunk(snap_id, index, data),
		}
		Ok(())
	}

	// Updates local trie with all registered main account and proxies
	pub fn update_storage_with_genesis_data(&mut self) -> Result<(), Error> {
		info!(target:"orderbook","Updating storage with genesis data");
		let data = self.runtime.runtime_api().get_all_accounts_and_proxies(&BlockId::number(
			self.last_finalized_block.saturated_into(),
		))?;
		let mut memory_db = self.memory_db.write();
		let mut working_state_root = self.working_state_root.write();
		let mut trie = Self::get_trie(&mut memory_db, &mut working_state_root);

		for (main, proxies) in data {
			// Register main and first proxy
			register_main(&mut trie, main.clone(), proxies[0].clone())?;
			// Register the remaining proxies
			if proxies.len() > 1 {
				for proxy in proxies.iter().skip(1) {
					add_proxy(&mut trie, main.clone(), proxy.clone())?;
				}
			} else {
				warn!(target:"orderbook","No proxies found for main: {:?}",main)
			}
		}
		// Commit the trie
		trie.commit();
		Ok(())
	}

	pub(crate) async fn handle_finality_notification(
		&mut self,
		notification: &FinalityNotification<B>,
	) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Finality notification for blk: {:?}", notification.header.number());
		let header = &notification.header;
		self.last_finalized_block = (*header.number()).saturated_into();
		// Check if snapshot should be generated or not
		if self.should_generate_snapshot() {
			let mut latest_stid = 0;
			{
				latest_stid = *self.latest_stid.read();
			}
			if let Err(err) = self.snapshot(latest_stid) {
				log::error!(target:"orderbook", "Couldn't generate snapshot: {:?}",err);
			} else {
				*self.last_block_snapshot_generated.write() = self.last_finalized_block;
			}
		}

		// We should not update latest summary if we are still syncing
		if !self.state_is_syncing {
			let latest_summary = self.runtime.runtime_api().get_latest_snapshot(
				&BlockId::Number(self.last_finalized_block.saturated_into()),
			)?;

			// Check if its genesis then update storage with genesis data
			if latest_summary.snapshot_id.is_zero() && self.latest_stid.read().is_zero() {
				info!(target: "orderbook", "📒 Loading genesis data from runtime ....");
				self.update_storage_with_genesis_data()?;
				// Update the latest snapshot summary.
				*self.last_snapshot.write() = latest_summary;
			}

			if let Some(orderbook_operator_public_key) =
				self.runtime.runtime_api().get_orderbook_opearator_key(&BlockId::number(
					self.last_finalized_block.saturated_into(),
				))? {
				info!(target:"orderbook","Orderbook operator public key found in runtime: {:?}",orderbook_operator_public_key);
				self.orderbook_operator_public_key = Some(orderbook_operator_public_key);
			} else {
				warn!(target:"orderbook","Orderbook operator public key not found in runtime");
			}
		}
		// if we are syncing the check progress
		if self.state_is_syncing {
			info!(target:"orderbook","Checking state sync progress");
			let mut inprogress: u16 = 0;
			let mut unavailable: u16 = 0;
			let total = self.sync_state_map.len();
			let last_summary = self.last_snapshot.read().clone();
			let mut missing_indexes = vec![];
			for (chunk_index, status) in self.sync_state_map.iter_mut() {
				match status {
					StateSyncStatus::Unavailable => {
						info!(target:"orderbook","Chunk: {:?} is unavailable",chunk_index);
						unavailable = unavailable.saturating_add(1);
						missing_indexes.push(*chunk_index);
					},
					StateSyncStatus::InProgress(who, when) => {
						info!(target:"orderbook","Chunk: {:?} is in progress with peer: {:?}",chunk_index,who);
						inprogress = inprogress.saturating_add(1);
						// If the peer has not responded with data in one minute we ask again
						if (Utc::now().timestamp() - *when) > 60 {
							missing_indexes.push(*chunk_index);
							warn!(target:"orderbook","Peer: {:?} has not responded with chunk: {:?}, asking someone else", who, chunk_index);
							*status = StateSyncStatus::Unavailable;
						}
					},
					StateSyncStatus::Available => {},
				}
			}
			info!(target:"orderbook","📒 State chunks sync status: inprogress: {:?}, unavailable: {:?}, total: {:?}",inprogress,unavailable,total);
			// If we have missing indexes, ask again to peers for these indexes
			if !missing_indexes.is_empty() {
				let message =
					GossipMessage::Want(last_summary.snapshot_id, prepare_bitmap(missing_indexes));
				let fullnodes = self
					.gossip_validator
					.fullnodes
					.read()
					.clone()
					.iter()
					.cloned()
					.collect::<Vec<PeerId>>();
				self.gossip_engine.send_message(fullnodes, message.encode());
			} else {
				// We have all the data, state is synced,
				// so load snapshot shouldn't have any problem now
				self.load_snapshot(&last_summary)?;
				self.state_is_syncing = false;
			}

			if !self.sync_oracle.is_major_syncing() & !self.state_is_syncing {
				// 1. Check if session change happened and we are part of the new set
				let active_set = self
					.runtime
					.runtime_api()
					.validator_set(&BlockId::number(self.last_finalized_block.saturated_into()))?
					.validators;
				if let Ok(signing_key) = self.get_validator_key(&active_set) {
					// 2. Check if the pending snapshot from previous set
					if let Some(pending_snaphot) = self.runtime.runtime_api().pending_snapshot(
						&BlockId::number(self.last_finalized_block.saturated_into()),
					)? {
						info!(target:"orderbook","Pending snapshot found: {:?}",pending_snaphot);
						// 3. if yes, then submit snapshot summaries for that.
						let offchain_storage = self
							.backend
							.offchain_storage()
							.ok_or(Error::OffchainStorageNotAvailable)?;

						match offchain_storage
							.get(ORDERBOOK_SNAPSHOT_SUMMARY_PREFIX, &pending_snaphot.encode())
						{
							None => {
								// This should never happen
								log::error!(target:"orderbook", "Unable to find snapshot summary for snapshot_id: {:?}",pending_snaphot)
							},
							Some(data) => {
								info!(target:"orderbook","Found snapshot summary for snapshot_id: {:?}",pending_snaphot);
								match SnapshotSummary::decode(&mut &data[..]) {
									Ok(mut summary) => {
										info!(target:"orderbook","Signing snapshot with: {:?}",hex::encode(signing_key.0));
										let signature = match bls_primitives::crypto::sign(
											&signing_key,
											&summary.sign_data(),
										) {
											Some(sig) => sig,
											None => {
												error!(target:"orderbook","📒 Failed to sign snapshot, not able to sign with validator key.");
												return Err(Error::SnapshotSigningFailed)
											},
										};

										summary.aggregate_signature = Some(signature);
										let bit_index = active_set
											.iter()
											.position(|v| v == &signing_key.into())
											.unwrap();
										set_bit_field(&mut summary.bitflags, bit_index as u16);
										// send it to runtime
										if self
                                            .runtime
                                            .runtime_api()
                                            .submit_snapshot(&BlockId::number(self.last_finalized_block.into()), summary)
                                            .expect("Something went wrong with the submit_snapshot runtime api; qed.").is_err()
                                        {
                                            error!(target:"orderbook","📒 Failed to submit snapshot to runtime");
                                            return Err(Error::FailedToSubmitSnapshotToRuntime);
                                        }
									},
									Err(err) => {
										// This should never happen
										log::error!(target:"orderbook", "Unable to decode snapshot summary for snapshotid: {:?}",err)
									},
								}
							},
						}
					}
				}
			}
		}

		let mut known_stids = self.known_messages.keys().collect::<Vec<&u64>>();

		known_stids.sort_unstable();

		info!(target:"engine", "Last processed Stid: {:?}, known keys: {:?}, next best stid: {:?}",
			self.latest_stid.read(),
			self.known_messages.len(),
		   known_stids.get(0)
		);
		Ok(())
	}

	/// Wait for Orderbook runtime pallet to be available.
	pub(crate) async fn wait_for_runtime_pallet(&mut self) {
		info!(target: "orderbook", "📒 Waiting for orderbook pallet to become available...");
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		while let Some(notif) = finality_stream.next().await {
			let at = BlockId::hash(notif.header.hash());
			if self.runtime.runtime_api().validator_set(&at).ok().is_some() {
				break
			} else {
				debug!(target: "orderbook", "📒 Waiting for orderbook pallet to become available...");
			}
		}
	}

	pub fn send_sync_requests(
		&mut self,
		summary: &SnapshotSummary<AccountId>,
	) -> Result<(), Error> {
		info!(target:"orderbook","📒 Sending sync requests for snapshot: {:?}",summary.snapshot_id);
		let offchain_storage =
			self.backend.offchain_storage().ok_or(Error::OffchainStorageNotAvailable)?;

		// Check the chunks we need
		// Store the missing chunk indexes
		let mut missing_chunks = vec![];
		for (index, chunk_hash) in summary.state_chunk_hashes.iter().enumerate() {
			if offchain_storage
				.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk_hash.encode().as_ref())
				.is_none()
			{
				missing_chunks.push(index as u16);
				self.sync_state_map.insert(index as u16, StateSyncStatus::Unavailable);
			} else {
				self.sync_state_map.insert(index as u16, StateSyncStatus::Available);
			}
		}
		// Prepare bitmap
		let bitmap = prepare_bitmap(missing_chunks);
		// Gossip the sync requests to all connected fullnodes
		let fullnodes = self
			.gossip_validator
			.fullnodes
			.read()
			.clone()
			.into_iter()
			.collect::<Vec<PeerId>>();
		let message = GossipMessage::Want(summary.snapshot_id, bitmap);
		self.gossip_engine.send_message(fullnodes, message.encode());
		Ok(())
	}

	/// Public method to get a mutable trie instance with the given mutable memory_db and
	/// working_state_root
	///
	/// # Parameters:
	/// - `memory_db`: a mutable reference to a MemoryDB instance
	/// - `working_state_root`: a mutable reference to a 32-byte array of bytes representing the
	///   root of the trie
	///
	/// # Returns
	/// `TrieDBMut`:  instance representing a mutable trie
	pub fn get_trie<'a>(
		memory_db: &'a mut MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>,
		working_state_root: &'a mut [u8; 32],
	) -> TrieDBMut<'a, ExtensionLayout> {
		let trie = if working_state_root == &mut [0u8; 32] {
			TrieDBMutBuilder::new(memory_db, working_state_root).build()
		} else {
			TrieDBMutBuilder::from_existing(memory_db, working_state_root).build()
		};
		trie
	}

	/// Loads the latest trading pair configs from runtime
	pub fn load_trading_pair_configs(&mut self) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Loading trading pair configs from runtime...");
		let tradingpairs = self
			.runtime
			.runtime_api()
			.read_trading_pair_configs(&BlockId::Number(self.client.info().finalized_number))?;

		for (pair, config) in tradingpairs {
			self.trading_pair_configs.insert(pair, config);
		}

		Ok(())
	}

	/// Main loop for Orderbook worker.
	///
	/// Wait for Orderbook runtime pallet to be available, then start the main async loop
	/// which is driven by gossiped user actions.
	pub(crate) async fn run(mut self) {
		info!(target: "orderbook", "📒 Orderbook worker started");
		self.wait_for_runtime_pallet().await;

		// Wait for blockchain sync to complete
		while self.sync_oracle.is_major_syncing() {
			info!(target: "orderbook", "📒 orderbook is not started waiting for blockhchain to sync completely");
			tokio::time::sleep(Duration::from_secs(12)).await;
		}

		// Get the latest summary from the runtime
		let latest_summary = match self
			.runtime
			.runtime_api()
			.get_latest_snapshot(&BlockId::Number(self.client.info().finalized_number))
		{
			Ok(summary) => summary,
			Err(err) => {
				error!(target:"orderbook","📒 Cannot get latest snapshot: {:?}",err);
				return
			},
		};
		{
			*self.last_snapshot.write() = latest_summary.clone();
		}
		// Lock, write and release
		info!(target:"orderbook","📒 Latest Snapshot state id: {:?}",latest_summary.state_change_id);
		// Try to load the snapshot from the database
		if let Err(err) = self.load_snapshot(&latest_summary) {
			warn!(target:"orderbook","📒 Cannot load snapshot from database: {:?}",err);
			info!(target:"orderbook","📒 Trying to sync snapshot from other peers");
			if let Err(err) = self.send_sync_requests(&latest_summary) {
				error!(target:"orderbook","Error while sending sync requests to peers: {:?}",err);
				return
			}
			self.state_is_syncing = true;
		}

		if let Err(err) = self.load_trading_pair_configs() {
			error!(target:"orderbook","Error while loading trading pair configs: {:?}",err);
			return
		}

		info!(target:"orderbook","📒 Starting event streams...");
		let mut gossip_messages = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					trace!(target: "orderbook", "📒 Got gossip message: {:?}", notification);
					match GossipMessage::decode(&mut &notification.message[..]).ok() {
						None => None,
						Some(msg) => Some((msg, notification.sender)),
					}
				})
				.fuse(),
		);
		// finality events stream
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		loop {
			let mut gossip_engine = &mut self.gossip_engine;
			futures::select_biased! {
				gossip = gossip_messages.next() => {
					if let Some((message,sender)) = gossip {
						// Gossip messages have already been verified to be valid by the gossip validator.
						if let Err(err) = self.process_gossip_message(&message,sender).await {
							error!(target: "orderbook", "📒 {}", err);
						}
					} else {
						return;
					}
				},
				message = self.message_sender_link.next() => {
					if let Some(message) = message {
						if let Err(err) = self.process_new_user_action(&message).await {
							error!(target: "orderbook", "📒 Error during user action import{:?}", err);
						}
					}else{
						return;
					}
				},
				finality = finality_stream.next() => {
					if let Some(finality) = finality {
						if let Err(err) = self.handle_finality_notification(&finality).await {
							error!(target: "orderbook", "📒 Error during finalized block import{}", err);
						}
					}else {
						error!(target:"orderbook","None finality recvd");
						return
					}
				},
				_ = gossip_engine => {
					error!(target: "orderbook", "📒 Gossip engine has terminated.");
					return;
				}
			}
		}
	}
}

/// The purpose of this function is to register a new main account along with a proxy account.
///
/// # Parameters
///
/// * `trie` - A mutable reference to a `TrieDBMut` with `ExtensionLayout`.
/// * `main` - The `AccountId` of the main account to be registered.
/// * `proxy` - The `AccountId` of the proxy account to be associated with the main account.
///
/// # Returns
///
/// Returns `Ok(())` if the registration is successful, or an `Error` if there was a problem
/// registering an account.
pub fn register_main(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	proxy: AccountId,
) -> Result<(), Error> {
	info!(target: "orderbook", "Registering main account: {:?}", main);
	if trie.contains(&main.encode())? {
		error!(target: "orderbook", "Main account already registered: {:?}", main);
		return Err(Error::MainAlreadyRegistered)
	}
	let account_info = AccountInfo { proxies: vec![proxy] };
	trie.insert(&main.encode(), &account_info.encode())?;
	Ok(())
}

/// The purpose of this function is to add new a proxy account to main account's list.
/// # Parameters
///
/// * `trie` - A mutable reference to a `TrieDBMut<ExtensionLayout>` instance, which represents the
///   trie database to modify.
/// * `main` - An `AccountId` representing the main account for which to add a proxy.
/// * `proxy` - An `AccountId` representing the proxy account to add to the list of authorized
///   proxies.
///
/// # Returns
///
/// Returns `Ok(())` on success, or an `Error` if there was a problem adding the proxy.
pub fn add_proxy(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	proxy: AccountId,
) -> Result<(), Error> {
	info!(target: "orderbook", "Adding proxy account: {:?}", proxy);
	match trie.get(&main.encode())? {
		Some(data) => {
			info!(target: "orderbook", "Main account found: {:?}", main);
			let mut account_info = AccountInfo::decode(&mut &data[..])?;
			if account_info.proxies.contains(&proxy) {
				return Err(Error::ProxyAlreadyRegistered)
			}
			account_info.proxies.push(proxy);
			trie.insert(&main.encode(), &account_info.encode())?;
		},
		None => return Err(Error::MainAccountNotFound),
	}
	Ok(())
}

/// The purpose of this function is to remove a proxy account from a main account's list.
///
/// # Parameters
///
/// * `trie` - A mutable reference to a `TrieDBMut<ExtensionLayout>` instance, which represents the
///   trie database to modify.
/// * `main` - An `AccountId` representing the main account for which to remove a proxy.
/// * `proxy` - An `AccountId` representing the proxy account that needs to be removed
///
/// # Returns
///
/// Returns `Ok(())` on success, or an `Error` if there was a problem removing the proxy account.
pub fn remove_proxy(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	proxy: AccountId,
) -> Result<(), Error> {
	info!(target: "orderbook", "Removing proxy account: {:?}", proxy);
	match trie.get(&main.encode())? {
		Some(data) => {
			let mut account_info = AccountInfo::decode(&mut &data[..])?;
			if account_info.proxies.contains(&proxy) {
				account_info
					.proxies
					.iter()
					.position(|x| *x == proxy)
					.map(|i| account_info.proxies.remove(i));
				trie.insert(&main.encode(), &account_info.encode())?;
			} else {
				return Err(Error::ProxyAccountNotFound)
			}
		},
		None => return Err(Error::MainAccountNotFound),
	}
	Ok(())
}

/// Deposits a specified amount of an asset into an account.
///
/// # Parameters
///
/// * `trie` - A mutable reference to a `TrieDBMut` object of type `ExtensionLayout`.
/// * `main` - An `AccountId` object representing the main account to deposit the asset into.
/// * `asset` - An `AssetId` object representing the asset to deposit.
/// * `amount` - A `Decimal` object representing the amount of the asset to deposit.
///
/// # Returns
///
/// A `Result<(), Error>` indicating whether the deposit was successful or not.
pub fn deposit(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	asset: AssetId,
	amount: Decimal,
) -> Result<(), Error> {
	info!(target: "orderbook", "Depositing asset: {:?}", asset);
	if !trie.contains(&main.encode())? {
		return Err(Error::MainAccountNotFound)
	}
	let account_asset = AccountAsset { main, asset };
	match trie.get(&account_asset.encode())? {
		Some(data) => {
			info!(target: "orderbook", "Account asset found: {:?}", account_asset);
			let mut balance = Decimal::decode(&mut &data[..])?;
			balance = balance.saturating_add(amount);
			trie.insert(&account_asset.encode(), &balance.encode())?;
		},
		None => {
			trie.insert(&account_asset.encode(), &amount.encode())?;
		},
	}
	Ok(())
}

/// Processes a trade between a maker and a taker, updating their order states and balances
/// accordingly.
///
/// # Arguments
///
/// * `trie` - A mutable reference to a `TrieDBMut` object of type `ExtensionLayout`.
/// * `trade` - A `Trade` object representing the trade to process.
///
/// # Returns
///
/// A `Result<(), Error>` indicating whether the trade was successfully processed or not.
pub fn process_trade(
	trie: &mut TrieDBMut<ExtensionLayout>,
	trade: Trade,
	config: TradingPairConfig,
) -> Result<(), Error> {
	info!(target: "orderbook", "Processing trade: {:?}", trade);
	if !trade.verify(config) {
		error!(target: "orderbook", "Trade verification failed");
		return Err(Error::InvalidTrade)
	}

	// Update balances
	let (maker_asset, maker_credit) = trade.credit(true);
	add_balance(trie, maker_asset, maker_credit)?;

	let (maker_asset, maker_debit) = trade.debit(true);
	sub_balance(trie, maker_asset, maker_debit)?;

	let (taker_asset, taker_credit) = trade.credit(false);
	add_balance(trie, taker_asset, taker_credit)?;

	let (taker_asset, taker_debit) = trade.debit(false);
	sub_balance(trie, taker_asset, taker_debit)?;
	Ok(())
}
