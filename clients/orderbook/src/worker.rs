use std::{
	collections::{BTreeMap, BTreeSet},
	marker::PhantomData,
	ops::Div,
	sync::Arc,
	time::Duration,
};

use chrono::Utc;
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use log::{debug, error, info, trace, warn};
use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{
		AccountAsset, AccountInfo, GossipMessage, ObMessage, StateSyncStatus, Trade, TradingPair,
		UserActions, WithdrawalRequest,
	},
	ObApi, SnapshotSummary, ValidatorSet, ORDERBOOK_SNAPSHOT_SUMMARY_PREFIX,
	ORDERBOOK_STATE_CHUNK_PREFIX, ORDERBOOK_WORKER_NONCE_PREFIX,
};
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
use polkadex_primitives::{
	ingress::IngressMessages,
	ocex::TradingPairConfig,
	utils::{prepare_bitmap, return_set_bits, set_bit_field},
	withdrawal::Withdrawal,
	AccountId, AssetId, BlockNumber,
};
use primitive_types::H128;
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::Decimal;
use sc_client_api::{Backend, FinalityNotification};
use sc_keystore::LocalKeystore;
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
	keystore::OrderbookKeyStore,
	metric_add, metric_inc, metric_set,
	metrics::Metrics,
	snapshot::SnapshotStore,
	utils::*,
	Client, DbRef,
};

pub(crate) struct WorkerParams<B: Block, BE, C, SO, N, R> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub sync_oracle: SO,
	pub metrics: Option<Metrics>,
	pub is_validator: bool,
	/// Local key store
	pub keystore: Option<Arc<LocalKeystore>>,
	pub message_sender_link: UnboundedReceiver<ObMessage>,
	/// Gossip network
	pub network: N,
	/// Chain specific Ob protocol name. See [`orderbook_protocol_name::standard_name`].
	pub protocol_name: sc_network::ProtocolName,
	pub _marker: PhantomData<B>,
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
	/// Local key store
	pub keystore: OrderbookKeyStore,
	gossip_engine: GossipEngine<B>,
	// gossip_validator: Arc<GossipValidator<B>>,
	// Last processed SnapshotSummary
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
	sync_state_map: BTreeMap<usize, StateSyncStatus>,
	// last block at which snapshot was generated
	last_block_snapshot_generated: Arc<RwLock<BlockNumber>>,
	// latest worker nonce
	latest_worker_nonce: Arc<RwLock<u64>>,
	latest_state_change_id: u64,
	// Map of trading pair configs
	trading_pair_configs: BTreeMap<TradingPair, TradingPairConfig>,
	pub(crate) orderbook_operator_public_key: Option<sp_core::ecdsa::Public>,
	// Our last snapshot waiting for approval
	pending_snapshot_summary: Option<SnapshotSummary<AccountId>>,
	// Fullnodes we are connected to
	fullnodes: Arc<RwLock<BTreeSet<PeerId>>>,
	last_processed_block_in_offchain_state: BlockNumber,
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
	/// Return a new Orderbook worker instance.
	///
	/// Note that a Orderbook worker is only fully functional if a corresponding
	/// Orderbook pallet has been deployed on-chain.
	///
	/// The Orderbook pallet is needed in order to keep track of the Orderbook authority set.
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, SO, N, R>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			sync_oracle,
			metrics,
			is_validator,
			keystore,
			message_sender_link,
			network,
			protocol_name,
			_marker,
			memory_db,
			working_state_root,
		} = worker_params;
		// Shared data
		let last_snapshot = Arc::new(RwLock::new(SnapshotSummary::default()));
		// Read from offchain state for new worker nonce
		let offchain_storage =
			backend.offchain_storage().expect("📒 Unable to load offchain storage");
		// if not found, set it to 0
		let nonce: u64 = match offchain_storage
			.get(ORDERBOOK_WORKER_NONCE_PREFIX, ORDERBOOK_WORKER_NONCE_PREFIX)
		{
			None => 0,
			Some(encoded_nonce) => {
				// Worker nonce stored using scale encoded fashion
				Decode::decode(&mut &encoded_nonce[..]).unwrap_or(0)
			},
		};
		let latest_worker_nonce = Arc::new(RwLock::new(nonce));
		let network = Arc::new(network);
		let fullnodes = Arc::new(RwLock::new(BTreeSet::new()));

		// Gossip Validator
		let gossip_validator = Arc::new(GossipValidator::new(
			latest_worker_nonce.clone(),
			fullnodes.clone(),
			is_validator,
			last_snapshot.clone(),
		));
		let gossip_engine =
			GossipEngine::new(network.clone(), protocol_name, gossip_validator, None);

		let keystore = OrderbookKeyStore::new(keystore);

		ObWorker {
			client,
			backend,
			runtime,
			sync_oracle,
			is_validator,
			_network: network,
			keystore,
			gossip_engine,
			// gossip_validator,
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
			last_block_snapshot_generated: Arc::new(RwLock::new(0)),
			latest_worker_nonce,
			latest_state_change_id: 0,
			trading_pair_configs: Default::default(),
			orderbook_operator_public_key: None,
			pending_snapshot_summary: None,
			fullnodes,
			last_processed_block_in_offchain_state: 0,
		}
	}

	/// The function checks whether a snapshot of the blockchain should be generated based on the
	/// pending withdrawals and block intervaland last stid
	///
	/// # Arguments
	/// * `&self`: a reference to an instance of a struct implementing some trait
	/// # Returns
	/// * `bool`: a boolean indicating whether a snapshot should be generated
	pub fn should_generate_snapshot(&self) -> bool {
		let at = BlockId::Number(self.last_finalized_block.saturated_into());
		// Get the snapshot generation intervals from the runtime API for the last finalized block
		let (pending_withdrawals_interval, block_interval) = self
			.runtime
			.runtime_api()
			.get_snapshot_generation_intervals(&at)
			.expect("📒 Expected the snapshot runtime api to be available, qed.");

		let last_accepted_worker_nonce: u64 = self
			.runtime
			.runtime_api()
			.get_last_accepted_worker_nonce(&BlockId::Number(self.client.info().best_number))
			.expect("📒Expected the snapshot runtime api to be available, qed.");
		// Check if a snapshot should be generated based on the pending withdrawals interval and
		// block interval
		if (pending_withdrawals_interval <= self.pending_withdrawals.len() as u64 ||
			block_interval <
				self.last_finalized_block
					.saturating_sub(*self.last_block_snapshot_generated.read())) &&
			last_accepted_worker_nonce < *self.latest_worker_nonce.read()
		// there is something new after last snapshot
		{
			info!(target:"orderbook", "📒 Snapshot should be generated");
			return true
		}
		// If a snapshot should not be generated, return false
		false
	}

	pub fn process_withdraw(
		&mut self,
		withdraw: WithdrawalRequest,
		worker_nonce: u64,
		state_change_id: u64,
	) -> Result<(), Error> {
		info!("📒 Processing withdrawal request: {:?}", withdraw);
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
		info!(target:"orderbook","📒 Queued withdrawal to pending list"); // Check if snapshot should be generated or not
		if self.should_generate_snapshot() {
			if let Err(err) = self.snapshot(worker_nonce, state_change_id) {
				log::error!(target:"orderbook", "📒Couldn't generate snapshot after reaching max pending withdrawals: {:?}",err);
				*self.last_block_snapshot_generated.write() = self.last_finalized_block;
			}
		}
		Ok(())
	}

	pub fn handle_blk_import(&mut self, num: BlockNumber) -> Result<(), Error> {
		info!("📒Handling block import: {:?}", num);
		if num.is_zero() {
			return Ok(())
		}
		let mut memory_db = self.memory_db.write();
		let mut working_state_root = self.working_state_root.write();
		info!("📒Starting state root: {:?}", hex::encode(working_state_root.clone()));
		// Get the ingress messages for this block
		let messages = self.runtime.runtime_api().ingress_messages(
			&BlockId::number(self.last_finalized_block.saturated_into()),
			num.saturated_into(),
		)?;

		{
			let mut trie = Self::get_trie(&mut memory_db, &mut working_state_root);
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
		info!("📒state root after processing: {:?}", hex::encode(working_state_root.clone()));
		self.last_processed_block_in_offchain_state = num;
		Ok(())
	}

	pub fn snapshot(&mut self, worker_nonce: u64, stid: u64) -> Result<(), Error> {
		info!(target:"orderbook","📒 Generating snapshot");
		let at = BlockId::number(self.last_finalized_block.saturated_into());
		let next_snapshot_id = self
			.runtime
			.runtime_api()
			.get_latest_snapshot(&at)?
			.snapshot_id
			.saturating_add(1);

		if let Some(pending_snapshot) = self.pending_snapshot_summary.as_ref() {
			if next_snapshot_id == pending_snapshot.snapshot_id {
				// We don't need to do anything because we already submitted the snapshot.
				return Ok(())
			}
		}
		let active_set = self.runtime.runtime_api().validator_set(&at)?;

		let mut summary = self.store_snapshot(worker_nonce, stid, next_snapshot_id, &active_set)?;
		if !self.is_validator {
			info!(target:"orderbook","📒 Not a validator, skipping snapshot signing.");
			// We are done if we are not a validator
			return Ok(())
		}

		let signing_key = self.keystore.get_local_key(&active_set.validators)?;
		info!(target:"orderbook","📒 Signing snapshot with: {:?}",signing_key);

		let signature = self.keystore.sign(&signing_key, &summary.sign_data())?;
		summary.aggregate_signature = Some(signature.into());
		let bit_index = active_set.validators().iter().position(|v| v == &signing_key).unwrap();
		info!(target:"orderbook","📒 Signing snapshot with bit index: {:?}",bit_index);
		set_bit_field(&mut summary.bitflags, bit_index);
		info!(target:"orderbook","📒 Signing snapshot with bit index: {:?}, signed auths: {:?}",bit_index,summary.signed_auth_indexes());
		// send it to runtime
		if self
			.runtime
			.runtime_api()
			.submit_snapshot(&BlockId::number(self.last_finalized_block.into()), summary.clone())?
			.is_err()
		{
			error!(target:"orderbook","📒 Failed to submit snapshot to runtime");
			return Err(Error::FailedToSubmitSnapshotToRuntime)
		}
		self.pending_snapshot_summary = Some(summary);
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
			UserActions::Withdraw(withdraw) =>
				self.process_withdraw(withdraw, action.worker_nonce, action.stid)?,
			UserActions::BlockImport(num) => self.handle_blk_import(num)?,
		}
		*self.latest_worker_nonce.write() = action.worker_nonce;
		info!(target:"orderbook","📒Updated working state root: {:?}",hex::encode(self.working_state_root.read().clone()));
		metric_set!(self, ob_snapshot_id, action.worker_nonce);
		self.latest_state_change_id = action.stid;
		// Multicast the message to other peers
		let gossip_message = GossipMessage::ObMessage(Box::new(action.clone()));
		self.gossip_engine.gossip_message(topic::<B>(), gossip_message.encode(), true);
		info!(target:"orderbook","📒Message with stid: {:?} gossiped to others",self.latest_worker_nonce.read());
		Ok(())
	}

	// Checks if we need to sync the orderbook state before processing the messages.
	pub async fn check_state_sync(&mut self) -> Result<(), Error> {
		info!(target:"orderbook","📒 Checking state sync");
		// X->Y sync: Ask peers to send the missed worker_nonec
		if !self.known_messages.is_empty() {
			info!(target:"orderbook","📒 Known messages len: {:?}", self.known_messages.len());
			// Collect all known worker nonces
			let mut known_worker_nonces = self.known_messages.keys().collect::<Vec<&u64>>();
			// Retain only those that are greater than what we already processed
			known_worker_nonces.retain(|x| **x > *self.latest_worker_nonce.read());
			known_worker_nonces.sort_unstable(); // unstable is fine since we know  worker nonces are unique
									 // if the next best known  worker nonces is not available then ask others
			if *known_worker_nonces[0] != self.latest_worker_nonce.read().saturating_add(1) {
				// Ask other peers to send us the requests  worker nonces.
				info!(target:"orderbook","📒 Asking peers to send us the missed \
                worker nonces: last processed nonce: {:?}, best known nonce: {:?} ",
                    self.latest_worker_nonce.read(), known_worker_nonces[0]);
				let message = GossipMessage::WantWorkerNonce(
					*self.latest_worker_nonce.read(),
					*known_worker_nonces[0],
				);

				self.gossip_engine.gossip_message(topic::<B>(), message.encode(), false);
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			} else {
				info!(target: "orderbook", "📒 sync request not required, we know the next worker_nonce");
			}
		} else {
			info!(target: "orderbook", "📒 No new messages known after worker_nonce: {:?}",self.latest_worker_nonce.read());
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
				info!(target: "orderbook", "📒 Loaded state from snapshot data ({} keys in memory db)",  store.map.len());
				let mut memory_db = self.memory_db.write();
				memory_db.load_from(store.convert_to_hashmap());
				info!(target: "orderbook", "📒 {} keys in loaded memory db",memory_db.data().len());
				let summary_clone = summary.clone();
				*self.last_snapshot.write() = summary_clone;
				*self.latest_worker_nonce.write() = summary.worker_nonce;
				self.latest_state_change_id = summary.state_change_id;
				self.last_processed_block_in_offchain_state = summary.last_processed_blk;
				*self.working_state_root.write() = summary.state_root.0;
				info!(target: "orderbook", "📒 0x{} state root loaded",hex::encode(summary.state_root.0));
			},
			Err(err) => {
				error!(target: "orderbook", "📒 Error decoding snapshot data: {err:?}");
				return Err(Error::Backend(format!("Error decoding snapshot data: {err:?}")))
			},
		}
		Ok(())
	}

	pub async fn process_new_user_action(&mut self, action: &ObMessage) -> Result<(), Error> {
		info!(target:"orderbook","📒 Received a new user action: {:?}",action);
		// Check if stid is newer or not
		if action.worker_nonce <= *self.latest_worker_nonce.read() {
			// Ignore stids we already know.
			warn!(target:"orderbook","📒 Ignoring old message: given: {:?}, latest stid: {:?}",action.worker_nonce,self.latest_worker_nonce.read());
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
		info!(target: "orderbook", "📒 Ob message recieved worker_nonce: {:?}",action.worker_nonce);
		// Cache the message
		self.known_messages.insert(action.worker_nonce, action.clone());
		if self.sync_oracle.is_major_syncing() | self.state_is_syncing {
			info!(target: "orderbook", "📒 Ob message cached for sync to complete: worker_nonce: {:?}",action.worker_nonce);
			return Ok(())
		}
		self.check_state_sync().await?;
		self.check_worker_nonce_gap_fill().await?;
		metric_set!(self, ob_worker_nonce, action.worker_nonce);
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
		worker_nonce: u64,
		state_change_id: u64,
		snapshot_id: u64,
		active_set: &ValidatorSet<AuthorityId>,
	) -> Result<SnapshotSummary<AccountId>, Error> {
		info!(target: "orderbook", "📒 Storing snapshot: {:?}", snapshot_id);
		if let Some(mut offchain_storage) = self.backend.offchain_storage() {
			// TODO: How to avoid cloning memory_db
			let store = SnapshotStore::new(self.memory_db.read().data().clone().into_iter());
			info!(target: "orderbook", "📒 snapshot contains {:?} keys", store.map.len());
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
						info!(target: "orderbook", "📒 Stored snapshot chunk: {}", chunk_hash);
						state_chunk_hashes.push(chunk_hash);
					}

					let withdrawals = self.pending_withdrawals.clone();
					self.pending_withdrawals.clear();
					info!(target: "orderbook", "📒 Stored snapshot withdrawals ({} bytes)", withdrawals.len());

					let working_state_root = self.working_state_root.read();

					let summary = SnapshotSummary {
						validator_set_id: active_set.set_id,
						snapshot_id,
						worker_nonce,
						state_root: working_state_root.clone().into(),
						state_change_id,
						bitflags: vec![0; active_set.len().div(128).saturating_add(1)],
						withdrawals,
						aggregate_signature: None,
						state_chunk_hashes,
						last_processed_blk: self.last_processed_block_in_offchain_state,
					};

					info!(target: "orderbook", "📒 Writing summary to offchain storage: {:?}", summary);

					offchain_storage.set(
						ORDERBOOK_SNAPSHOT_SUMMARY_PREFIX,
						&snapshot_id.encode(),
						&summary.encode(),
					);
					// Store the last processed worker nonce too
					offchain_storage.set(
						ORDERBOOK_WORKER_NONCE_PREFIX,
						ORDERBOOK_WORKER_NONCE_PREFIX,
						&summary.worker_nonce.encode(),
					);
					Ok(summary)
				},
				Err(err) => Err(Error::Backend(format!("📒 Error serializing the data: {err:?}"))),
			}
		}
		Err(Error::Backend("📒 Offchain Storage not Found".parse().unwrap()))
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
				match offchain_storage.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk_hash.0.as_ref()) {
					None =>
						error!(target:"orderbook","📒 Unable to find chunk from offchain state: {:?}",chunk_hash),
					Some(mut chunk) => {
						let computed_hash = H128::from(blake2_128(&chunk));
						if computed_hash != *chunk_hash {
							warn!(target:"orderbook","📒 orderbook state hash mismatch: computed: {:?}, expected: {:?}",computed_hash,chunk_hash);
							return Err(Error::StateHashMisMatch)
						}
						data.append(&mut chunk);
					},
				}
			}
			self.load_state_from_data(&data, summary)?;
		} else {
			warn!(target:"orderbook","📒 orderbook state chunk not found");
		}
		Ok(())
	}

	/// go through the `known_messages` incrementally starting from the last snapshot's
	/// worker_nonce and process the messages. if a message is not found, it means ?
	pub async fn check_worker_nonce_gap_fill(&mut self) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Checking for worker_nonce gap fill");
		let mut next_worker_nonce = self.latest_worker_nonce.read().saturating_add(1);
		while let Some(action) = self.known_messages.get(&next_worker_nonce).cloned() {
			if let Err(err) = self.handle_action(&action) {
				match err {
					Error::Keystore(_) =>
						error!(target:"orderbook","📒 BLS session key not found: {:?}",err),
					_ => {
						error!(target:"orderbook","📒 Error processing action: {:?}",err);
						// The node found an error during processing of the action. This means
						// We need to revert everything after the last successful snapshot
						// Clear the working state
						self.memory_db.write().clear();
						info!(target:"orderbook","📒 Working state cleared.");
						// We forget about everything else from cache.
						self.known_messages.clear();
						info!(target:"orderbook","📒 OB messages cache cleared.");
						let latest_summary = self.runtime.runtime_api().get_latest_snapshot(
							&BlockId::Number(self.last_finalized_block.saturated_into()),
						)?;
						self.load_snapshot(&latest_summary)?;
						return Ok(())
					},
				}
			}
			next_worker_nonce = self.latest_worker_nonce.read().saturating_add(1);
		}
		Ok(())
	}

	/// Checks the local `known_messages` to see if we have any messages between the `from` and `to`
	/// worker_nonce. If we do, we gossip the `WorkerNonces` message to the peer that requested it.
	///
	/// # Arguments
	/// * `from` - The worker_nonce to start from
	/// * `to` - The worker_nonce to end at
	/// * `peer` - The peer that requested the worker_nonces
	///
	/// # Returns
	/// * `()` - No return value
	pub fn want_worker_nonce(&mut self, from: &u64, to: &u64, peer: Option<PeerId>) {
		info!(target: "orderbook", "📒 Want worker_nonce: {:?} - {:?}", from, to);
		if let Some(peer) = peer {
			info!(target: "orderbook", "📒 Sending worker_nonce request to peer: {:?}", peer);
			let gossip_messages = self.get_want_worker_nonce_messages(from, to);
			for message in &gossip_messages {
				self.gossip_engine.send_message(vec![peer], message.encode());
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			}
		}
	}
	/// Returns a list of gossip messages that contain the worker_nonces requested
	/// from the `from` to the `to` worker_nonce.
	/// The messages are limited to 10MB in size.
	///
	/// # Arguments:
	///   * `from`: The first worker_nonce requested
	///   * `to`: The last worker_nonce requested
	/// # returns:
	///   A list of gossip messages that contain the worker_nonces requested
	pub fn get_want_worker_nonce_messages(&self, from: &u64, to: &u64) -> Vec<Vec<u8>> {
		let mut gossip_messages = vec![];
		let mut messages = vec![];
		for worker_nonce in *from..=*to {
			// We dont allow gossip messsages to be greater than 10MB
			if messages.encoded_size() >= 10 * 1024 * 1024 {
				// If we reach size limit, we send data in chunks of 10MB.
				info!(target: "orderbook", "📒 Sending worker_nonce chunk: {:?}", messages.len());
				let message = GossipMessage::WorkerNonces(Box::new(messages));
				gossip_messages.push(message.encode());
				messages = vec![] // Reset the buffer
			}
			info!(target:"test", "📒 known messages length:{:?}", self.known_messages.len());
			if let Some(msg) = self.known_messages.get(&worker_nonce) {
				info!(target: "test", "📒 known_messages: {:?}", self.known_messages.len());
				messages.push(msg.clone());
			}
		}
		// Send the final chunk if any
		if !messages.is_empty() {
			let message = GossipMessage::WorkerNonces(Box::new(messages));
			gossip_messages.push(message.encode());
		}
		gossip_messages
	}
	/// Handles the worker_nonces received via gossip.
	/// The worker_nonces are stored in the `known_messages` map.
	///
	/// # Arguments
	///  * `messages`: The list of worker_nonces received via gossip
	/// # returns:
	/// Ok(()) if the worker_nonces are stored successfully
	pub async fn got_worker_nonces_via_gossip(
		&mut self,
		messages: &Vec<ObMessage>,
	) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Got worker_nonces via gossip: {:?}", messages.len());
		for message in messages {
			// TODO: handle reputation change.
			self.known_messages.entry(message.worker_nonce).or_insert(message.clone());
		}
		self.check_worker_nonce_gap_fill().await
	}

	// Expects the set bits in the bitmap to be missing chunks
	pub async fn want(&mut self, snapshot_id: &u64, bitmap: &Vec<u128>, remote: Option<PeerId>) {
		info!(target: "orderbook", "📒 Want snapshot: {:?} - {:?}", snapshot_id, bitmap);
		// Don't respond to want request if we are syncing
		if self.state_is_syncing {
			info!(target: "orderbook", "📒 we are syncing, Want request for id {snapshot_id:?} from {remote:?} ignored");
			return
		}

		if let Some(peer) = remote {
			let mut chunks_we_have = vec![];
			let mut highest_index = 0;
			let at = BlockId::Number(self.last_finalized_block.saturated_into());
			if let Ok(Some(summary)) =
				self.runtime.runtime_api().get_snapshot_by_id(&at, *snapshot_id)
			{
				if let Some(offchain_storage) = self.backend.offchain_storage() {
					let required_indexes: Vec<usize> = return_set_bits(bitmap);
					for chunk_index in required_indexes {
						if offchain_storage
							.get(
								ORDERBOOK_STATE_CHUNK_PREFIX,
								summary.state_chunk_hashes[chunk_index].encode().as_ref(),
							)
							.is_some()
						{
							chunks_we_have.push(chunk_index);
							highest_index = chunk_index.max(highest_index);
						}
					}
				}
			} else {
				// TODO: Reduce reputation if else block is happens
			}

			if !chunks_we_have.is_empty() {
				let message = GossipMessage::Have(
					*snapshot_id,
					prepare_bitmap(&chunks_we_have, highest_index)
						.expect("📒 Expected to create bitmap"),
				);
				self.gossip_engine.send_message(vec![peer], message.encode());
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			}
		}
	}

	pub async fn have(&mut self, snapshot_id: &u64, bitmap: &Vec<u128>, remote: Option<PeerId>) {
		info!(target: "orderbook", "📒 Have snapshot: {:?} - {:?}", snapshot_id, bitmap);
		if let Some(peer) = remote {
			// Note: Set bits here are available for syncing
			let available_chunks: Vec<usize> = return_set_bits(bitmap);
			let mut want_chunks = vec![];
			let mut highest_index = 0; // We need the highest index to prepare the bitmap for index in available_chunks {
			for index in available_chunks {
				if let Some(chunk_status) = self.sync_state_map.get_mut(&index) {
					if *chunk_status == StateSyncStatus::Unavailable {
						want_chunks.push(index);
						highest_index = index.max(highest_index);
						*chunk_status = StateSyncStatus::InProgress(peer, Utc::now().timestamp());
					}
				}
			}

			if !want_chunks.is_empty() {
				let message = GossipMessage::RequestChunk(
					*snapshot_id,
					prepare_bitmap(&want_chunks, highest_index)
						.expect("📒 Expected to create bitmap"),
				);
				debug!(target: "orderbook", "📒 requested chunk of {:?} to {:?}", snapshot_id, peer);
				self.gossip_engine.send_message(vec![peer], message.encode());
				metric_inc!(self, ob_messages_sent);
				metric_add!(self, ob_data_sent, message.encoded_size() as u64);
			}
		}
	}

	pub async fn request_chunk(
		&mut self,
		snapshot_id: &u64,
		bitmap: &Vec<u128>,
		remote: Option<PeerId>,
	) {
		info!(target: "orderbook", "📒 Request chunk: {:?}, {:?}, {:?}", snapshot_id, bitmap, remote);
		if let Some(peer) = remote {
			if let Some(offchian_storage) = self.backend.offchain_storage() {
				let at = BlockId::Number(self.last_finalized_block.saturated_into());
				if let Ok(Some(summary)) =
					self.runtime.runtime_api().get_snapshot_by_id(&at, *snapshot_id)
				{
					let chunk_indexes: Vec<usize> = return_set_bits(bitmap);
					for index in chunk_indexes {
						match summary.state_chunk_hashes.get(index) {
							None => {
								log::warn!(target:"orderbook","📒 Chunk hash not found for index: {:?}",index)
							},
							Some(chunk_hash) => {
								if let Some(data) = offchian_storage
									.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk_hash.0.as_ref())
								{
									let message =
										GossipMessage::Chunk(*snapshot_id, index as u16, data);
									debug!(target: "orderbook", "📒 Chunk message size: {:?}", message.encode().len());
									self.gossip_engine.send_message(vec![peer], message.encode());
									info!(target: "orderbook", "📒 Chunk {:?} of {:?} sent to {:?}", chunk_hash, snapshot_id, remote);
									metric_inc!(self, ob_messages_sent);
									metric_add!(self, ob_data_sent, message.encoded_size() as u64);
								} else {
									warn!(target:"orderbook","📒 No chunk found for index: {:?}",index)
								}
							},
						}
					}
				} else {
					warn!(target:"orderbook","📒 No snapshot found for request chunk")
				}
			} else {
				warn!(target:"orderbook","📒 No offchain storage found for request chunk")
			}
		} else {
			warn!(target:"orderbook","📒 No peer found for request chunk")
		}
	}

	pub fn process_chunk(&mut self, snapshot_id: &u64, index: &usize, data: &[u8]) {
		info!(target: "orderbook", "📒 Chunk snapshot: {:?} - {:?} - {:?}", snapshot_id, index, data.len());
		if let Some(mut offchian_storage) = self.backend.offchain_storage() {
			let at = BlockId::Number(self.last_finalized_block.saturated_into());
			if let Ok(Some(summary)) =
				self.runtime.runtime_api().get_snapshot_by_id(&at, *snapshot_id)
			{
				match summary.state_chunk_hashes.get(*index) {
					None =>
						warn!(target:"orderbook","📒 Invalid index recvd, index > length of state chunk hashes"),
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
						} else {
							log::warn!(target:"orderbook","📒 Invalid chunk hash, dropping chunk...");
						}
					},
				}
			}
		}
	}

	pub async fn process_gossip_message(
		&mut self,
		message: &GossipMessage,
		remote: Option<PeerId>,
	) -> Result<(), Error> {
		info!(target:"orderbook","📒 Processing gossip message: {:?}",message);
		metric_inc!(self, ob_messages_recv);
		metric_add!(self, ob_data_recv, message.encoded_size() as u64);
		match message {
			GossipMessage::WantWorkerNonce(from, to) => self.want_worker_nonce(from, to, remote),
			GossipMessage::WorkerNonces(messages) =>
				self.got_worker_nonces_via_gossip(messages).await?,
			GossipMessage::ObMessage(msg) => self.process_new_user_action(msg).await?,
			GossipMessage::Want(snap_id, bitmap) => self.want(snap_id, bitmap, remote).await,
			GossipMessage::Have(snap_id, bitmap) => self.have(snap_id, bitmap, remote).await,
			GossipMessage::RequestChunk(snap_id, bitmap) =>
				self.request_chunk(snap_id, bitmap, remote).await,
			GossipMessage::Chunk(snap_id, index, data) =>
				self.process_chunk(snap_id, &(*index).into(), data),
		}
		Ok(())
	}

	// Updates local trie with all registered main account and proxies
	pub fn update_storage_with_genesis_data(&mut self) -> Result<(), Error> {
		info!(target:"orderbook","📒 Updating storage with genesis data");
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
				debug!(target:"orderbook","📒 Only one proxy found for main: {:?}",main)
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
		let active_set = self
			.runtime
			.runtime_api()
			.validator_set(&BlockId::number(self.last_finalized_block.saturated_into()))?
			.validators;
		if let Err(err) = self.keystore.get_local_key(&active_set) {
			log::error!(target:"orderbook","📒 No BLS key found: {:?}",err);
		} else {
			log::info!(target:"orderbook","📒 Active BLS key found")
		}
		// Check if snapshot should be generated or not
		if self.should_generate_snapshot() {
			let latest_worker_nonce = *self.latest_worker_nonce.read();
			if let Err(err) = self.snapshot(latest_worker_nonce, self.latest_state_change_id) {
				log::error!(target:"orderbook", "📒 Couldn't generate snapshot after reaching max blocks limit: {:?}",err);
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
			if latest_summary.snapshot_id.is_zero() && self.latest_worker_nonce.read().is_zero() {
				info!(target: "orderbook", "📒 Loading genesis data from runtime ....");
				self.update_storage_with_genesis_data()?;
				// Update the latest snapshot summary.
				*self.last_snapshot.write() = latest_summary;
			} else {
				let last_worker_nonce = latest_summary.worker_nonce;
				// There is a valid snapshot from runtime, so update our state.
				*self.last_snapshot.write() = latest_summary;
				// Prune the known messages cache
				// Remove all worker nonces older than the last processed worker nonce
				self.known_messages.retain(|k, _| *k > last_worker_nonce);
			}
			if let Some(orderbook_operator_public_key) =
				self.runtime.runtime_api().get_orderbook_opearator_key(&BlockId::number(
					self.last_finalized_block.saturated_into(),
				))? {
				info!(target:"orderbook","📒 Orderbook operator public key found in runtime: {:?}",orderbook_operator_public_key);
				self.orderbook_operator_public_key = Some(orderbook_operator_public_key);
			} else {
				warn!(target:"orderbook","📒 Orderbook operator public key not found in runtime");
			}
		}
		// if we are syncing the check progress
		if self.state_is_syncing {
			info!(target:"orderbook","📒 Checking state sync progress");
			let mut inprogress: u16 = 0;
			let mut unavailable: u16 = 0;
			let total = self.sync_state_map.len();
			let last_summary = self.last_snapshot.read().clone();
			let mut missing_indexes = vec![];
			let mut highest_missing_index = 0;
			for (chunk_index, status) in self.sync_state_map.iter_mut() {
				match status {
					StateSyncStatus::Unavailable => {
						info!(target:"orderbook","📒 Chunk: {:?} is unavailable",chunk_index);
						unavailable = unavailable.saturating_add(1);
						missing_indexes.push(*chunk_index);
						highest_missing_index = (*chunk_index).max(highest_missing_index);
					},
					StateSyncStatus::InProgress(who, when) => {
						info!(target:"orderbook","📒 Chunk: {:?} is in progress with peer: {:?}",chunk_index,who);
						inprogress = inprogress.saturating_add(1);
						// If the peer has not responded with data in one minute we ask again
						if (Utc::now().timestamp() - *when) > 60 {
							missing_indexes.push(*chunk_index);
							highest_missing_index = (*chunk_index).max(highest_missing_index);
							warn!(target:"orderbook","📒 Peer: {:?} has not responded with chunk: {:?}, asking someone else", who, chunk_index);
							*status = StateSyncStatus::Unavailable;
						}
					},
					StateSyncStatus::Available => {},
				}
			}
			info!(target:"orderbook","📒 State chunks sync status: inprogress: {:?}, unavailable: {:?}, total: {:?}",inprogress,unavailable,total);
			// If we have missing indexes, ask again to peers for these indexes
			if !missing_indexes.is_empty() {
				let message = GossipMessage::Want(
					last_summary.snapshot_id,
					prepare_bitmap(&missing_indexes, highest_missing_index)
						.expect("📒 Expected to create bitmap"),
				);
				let fullnodes =
					self.fullnodes.read().clone().iter().cloned().collect::<Vec<PeerId>>();
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
				if let Ok(signing_key) = self.keystore.get_local_key(&active_set) {
					// 2. Check if the pending snapshot from previous set
					if let Some(pending_snaphot) = self.runtime.runtime_api().pending_snapshot(
						&BlockId::number(self.last_finalized_block.saturated_into()),
						signing_key.clone(),
					)? {
						info!(target:"orderbook","📒 Pending snapshot found: {:?}",pending_snaphot);
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
								log::error!(target:"orderbook", "📒 Unable to find snapshot summary for snapshot_id: {:?}",pending_snaphot)
							},
							Some(data) => {
								info!(target:"orderbook","📒 Loading snapshot summary for snapshot_id: {:?} from off chain storage",pending_snaphot);
								match SnapshotSummary::decode(&mut &data[..]) {
									Ok(mut summary) => {
										info!(target:"orderbook","📒 Signing snapshot with: {:?}",signing_key);
										let signature = self
											.keystore
											.sign(&signing_key, &summary.sign_data())?;
										summary.aggregate_signature = Some(signature.into());
										let bit_index = active_set
											.iter()
											.position(|v| v == &signing_key)
											.unwrap();
										set_bit_field(&mut summary.bitflags, bit_index);

										if self.pending_snapshot_summary != Some(summary.clone()) {
											// send it to runtime
											if self
												.runtime
												.runtime_api()
												.submit_snapshot(
													&BlockId::number(
														self.last_finalized_block.into(),
													),
													summary.clone(),
												)?
												.is_err()
											{
												error!(target:"orderbook","📒 Failed to submit snapshot to runtime");
												return Err(Error::FailedToSubmitSnapshotToRuntime)
											}
											self.pending_snapshot_summary = Some(summary);
										} else {
											log::debug!(target:"orderbook", "📒We already submitted snapshot: {:?}",self.pending_snapshot_summary);
										}
									},
									Err(err) => {
										// This should never happen
										log::error!(target:"orderbook", "📒 Unable to decode snapshot summary for snapshotid: {:?}",err)
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

		info!(target:"orderbook", "📒 Last processed worker nonce: {:?}, cached messages: {:?}, next best worker nonce: {:?}",
			self.latest_worker_nonce.read(),
			self.known_messages.len(),
		   known_stids.get(0)
		);
		if let Some(to) = known_stids.get(0) {
			let from = *self.latest_worker_nonce.read();
			// Send it only if we are missing any messages
			if to.saturating_sub(from) > 1 {
				let want_request = GossipMessage::WantWorkerNonce(from, **to);
				self.gossip_engine.gossip_message(topic::<B>(), want_request.encode(), true);
				info!(target:"orderbook","📒 Sending periodic sync request for nonces between: from:{from:?} to: {to:?}");
			} else if to.saturating_sub(from) == 1 && !self.is_validator {
				// If we are a fullnode and we know all the stids
				// then broadcast the next best nonce periodically
				// Unwrap is fine because we know the message exists
				let best_msg = GossipMessage::ObMessage(Box::new(
					self.known_messages.get(to).cloned().unwrap(),
				));
				self.gossip_engine.gossip_message(topic::<B>(), best_msg.encode(), true);
				self.gossip_engine.broadcast_topic(topic::<B>(), true);
				info!(target:"orderbook","📒 Sending periodic best message broadcast, nonce: {to:?}");
			}
		}
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
		let mut highest_chunk_index = 0;
		for (index, chunk_hash) in summary.state_chunk_hashes.iter().enumerate() {
			if offchain_storage
				.get(ORDERBOOK_STATE_CHUNK_PREFIX, chunk_hash.encode().as_ref())
				.is_none()
			{
				missing_chunks.push(index);
				highest_chunk_index = index.max(highest_chunk_index);
				self.sync_state_map.insert(index, StateSyncStatus::Unavailable);
			} else {
				self.sync_state_map.insert(index, StateSyncStatus::Available);
			}
		}
		// Prepare bitmap
		let bitmap = prepare_bitmap(&missing_chunks, highest_chunk_index)
			.expect("📒 Expected to create bitmap");
		// Gossip the sync requests to all connected nodes
		let message = GossipMessage::Want(summary.snapshot_id, bitmap);
		self.gossip_engine.gossip_message(topic::<B>(), message.encode(), false);
		info!(target:"orderbook","📒 State sync request send to connected peers ");
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
			info!(target: "orderbook", "📒 Creating a new trie as state root is empty");
			TrieDBMutBuilder::new(memory_db, working_state_root).build()
		} else {
			trace!(target: "orderbook", "📒 Loading trie from existing Db and state root");
			TrieDBMutBuilder::from_existing(memory_db, working_state_root).build()
		};
		trie
	}

	/// Loads the latest trading pair configs from runtime
	pub fn load_trading_pair_configs(&mut self, blk_num: BlockNumber) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Loading trading pair configs from runtime...");
		let tradingpairs = self
			.runtime
			.runtime_api()
			.read_trading_pair_configs(&BlockId::Number(blk_num.saturated_into()))?;
		info!(target: "orderbook","Loaded {:?} trading pairs", tradingpairs.len());
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
			info!(target: "orderbook", "📒 orderbook is not started waiting for blockchain to sync completely");
			tokio::time::sleep(Duration::from_secs(12)).await;
		}

		if let Ok(public_key) = self
			.runtime
			.runtime_api()
			.get_orderbook_opearator_key(&BlockId::Number(self.client.info().finalized_number))
		{
			self.orderbook_operator_public_key = public_key;
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
		info!(target:"orderbook","📒 Latest Snapshot state id: {:?}",latest_summary.worker_nonce);
		// Try to load the snapshot from the database
		if let Err(err) = self.load_snapshot(&latest_summary) {
			warn!(target:"orderbook","📒 Cannot load snapshot from database: {:?}",err);
			info!(target:"orderbook","📒 Trying to sync snapshot from other peers");
			if let Err(err) = self.send_sync_requests(&latest_summary) {
				error!(target:"orderbook","📒 Error while sending sync requests to peers: {:?}",err);
				return
			}
			self.state_is_syncing = true;
		}

		if let Err(err) =
			self.load_trading_pair_configs(self.client.info().finalized_number.saturated_into())
		{
			error!(target:"orderbook","📒 Error while loading trading pair configs: {:?}",err);
			return
		}

		info!(target:"orderbook","📒 Starting event streams...");
		let mut gossip_messages = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					match GossipMessage::decode(&mut &notification.message[..]).ok() {
						None => {
							warn!(target: "orderbook", "📒 Gossip message decode failed: {:?}", notification);
							None
						},
						Some(msg) => {
							trace!(target: "orderbook", "📒 Got gossip message: {:?}", msg);
							Some((msg, notification.sender))
						},
					}
				})
				.fuse(),
		);
		// finality events stream
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		loop {
			let mut gossip_engine = &mut self.gossip_engine;
			futures::select_biased! {
				_ = gossip_engine => {
					error!(target: "orderbook", "📒 Gossip engine has terminated.");
					return;
				}
				finality = finality_stream.next() => {
					if let Some(finality) = finality {
						if let Err(err) = self.handle_finality_notification(&finality).await {
							error!(target: "orderbook", "📒 Error during finalized block import{}", err);
						}
					}else {
						error!(target:"orderbook","📒 None finality received");
						return
					}
				},
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
	info!(target: "orderbook", "📒 Registering main account: {:?}", main);
	if trie.contains(&main.encode())? {
		warn!(target: "orderbook", "📒 Main account already registered: {:?}", main);
		return Ok(())
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
	info!(target: "orderbook", "📒Adding proxy account: {:?}", proxy);
	match trie.get(&main.encode())? {
		Some(data) => {
			info!(target: "orderbook", "📒 Main account found: {:?}", main);
			let mut account_info = AccountInfo::decode(&mut &data[..])?;
			if account_info.proxies.contains(&proxy) {
				warn!(target: "orderbook", "📒 Proxy account already registered: {:?}", proxy);
				return Ok(())
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
	info!(target: "orderbook", "📒 Removing proxy account: {:?}", proxy);
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
	info!(target: "orderbook", "📒 Depositing asset: {:?}", asset);
	if !trie.contains(&main.encode())? {
		return Err(Error::MainAccountNotFound)
	}
	let account_asset = AccountAsset { main, asset };
	match trie.get(&account_asset.encode())? {
		Some(data) => {
			info!(target: "orderbook", "📒 Account asset found: {:?}", account_asset);
			let mut balance = Decimal::decode(&mut &data[..])?;
			balance = balance.saturating_add(amount);
			trie.insert(&account_asset.encode(), &balance.encode())?;
		},
		None => {
			info!(target: "orderbook", "📒 Account asset created: {:?}", account_asset);
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
	info!(target: "orderbook", "📒 Processing trade: {:?}", trade);
	if !trade.verify(config) {
		error!(target: "orderbook", "📒 Trade verification failed");
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
