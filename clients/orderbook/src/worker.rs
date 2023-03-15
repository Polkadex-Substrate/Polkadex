use std::{
	borrow::Cow,
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	sync::Arc,
};

use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use log::{debug, error, info, trace, warn};
use memory_db::{HashKey, MemoryDB};
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::Decimal;
use sc_client_api::{Backend, FinalityNotification};
use sc_network::PeerId;
use sc_network_common::{
	protocol::event::Event,
	service::{NetworkNotification, NotificationSender},
};
use sc_network_gossip::{GossipEngine, Network as GossipNetwork};
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::{SaturatedConversion, Saturating};
use sp_consensus::SyncOracle;
use sp_core::{crypto::AccountId32, offchain::OffchainStorage, H256};
use sp_runtime::{
	generic::BlockId,
	traits::{Block, Header},
};
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

use bls_primitives::{Public, Signature};
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{
		AccountAsset, AccountInfo, ObMessage, OrderState, Trade, UserActions, WithdrawalRequest,
	},
	utils::set_bit_field,
	ObApi, SnapshotSummary, StidImportRequest, StidImportResponse,
};
use polkadex_primitives::{
	ingress::IngressMessages, withdrawal::Withdrawal, AccountId, AssetId, BlockNumber,
};

use crate::{
	error::Error,
	gossip::{topic, GossipValidator},
	metrics::Metrics,
	utils::*,
	Client,
};

pub const STID_IMPORT_REQUEST: &str = "stid_request";
pub const STID_IMPORT_RESPONSE: &str = "stid_response";

pub(crate) struct WorkerParams<B: Block, BE, C, SO, N> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub sync_oracle: SO,
	// pub key_store: BeefyKeystore,
	// pub links: BeefyVoterLinks<B>,
	pub metrics: Option<Metrics>,
	pub message_sender_link: UnboundedReceiver<ObMessage>,
	/// Gossip network
	pub network: N,
	/// Chain specific Ob protocol name. See [`orderbook_protocol_name::standard_name`].
	pub protocol_name: std::borrow::Cow<'static, str>,
	pub _marker: PhantomData<B>,
}

/// A Orderbook worker plays the Orderbook protocol
pub(crate) struct ObWorker<B: Block, BE, C, SO, N> {
	// utilities
	client: Arc<C>,
	backend: Arc<BE>,
	sync_oracle: SO,
	network: Arc<N>,
	// key_store: BeefyKeystore,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	// Last processed state change id
	last_snapshot: Arc<RwLock<SnapshotSummary>>,
	// Working state root,
	working_state_root: [u8; 32],
	// Known state ids
	known_messages: BTreeMap<u64, ObMessage>,
	// Links between the block importer, the background voter and the RPC layer.
	// links: BeefyVoterLinks<B>,
	pending_withdrawals: Vec<Withdrawal<AccountId>>,
	// voter state
	/// Orderbook client metrics.
	metrics: Option<Metrics>,
	message_sender_link: UnboundedReceiver<ObMessage>,
	_marker: PhantomData<N>,
	// In memory store
	memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>,
	// Last finalized block
	last_finalized_block: BlockNumber,
}

impl<B, BE, C, SO, N> ObWorker<B, BE, C, SO, N>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE> + ProvideRuntimeApi<B>,
	C::Api: ObApi<B>,
	SO: Send + Sync + Clone + 'static + SyncOracle,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
{
	/// Return a new BEEFY worker instance.
	///
	/// Note that a BEEFY worker is only fully functional if a corresponding
	/// BEEFY pallet has been deployed on-chain.
	///
	/// The BEEFY pallet is needed in order to keep track of the BEEFY authority set.
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, SO, N>) -> Self {
		let WorkerParams {
			client,
			backend,
			// key_store,
			sync_oracle,
			// links,
			metrics,
			message_sender_link,
			network,
			protocol_name,
			_marker,
		} = worker_params;

		let last_snapshot = Arc::new(RwLock::new(SnapshotSummary::default()));
		let network = Arc::new(network);
		let gossip_validator = Arc::new(GossipValidator::new(last_snapshot.clone()));
		let gossip_engine =
			GossipEngine::new(network.clone(), protocol_name, gossip_validator.clone(), None);

		ObWorker {
			client: client.clone(),
			backend,
			sync_oracle,
			// key_store,
			network,
			gossip_engine,
			gossip_validator,
			memory_db: MemoryDB::default(),
			// links,
			message_sender_link,
			metrics,
			last_snapshot,
			_marker: Default::default(),
			known_messages: Default::default(),
			working_state_root: Default::default(),
			pending_withdrawals: vec![],
			last_finalized_block: 0,
		}
	}

	pub fn process_trade(&mut self, trade: Trade) -> Result<(), Error> {
		let Trade { maker, taker, price, amount } = trade.clone();
		let mut trie =
			TrieDBMutBuilder::from_existing(&mut self.memory_db, &mut self.working_state_root)
				.build();
		// Check order states
		let maker_order_state = match trie.get(maker.id.as_ref())? {
			None => OrderState::from(&maker),
			Some(data) => {
				let mut state = OrderState::decode(&mut &data[..])?;
				if !state.update(&maker, price, amount) {
					return Err(Error::OrderStateCheckFailed)
				}
				state
			},
		};

		let taker_order_state = match trie.get(taker.id.as_ref())? {
			None => OrderState::from(&taker),
			Some(data) => {
				let mut state = OrderState::decode(&mut &data[..])?;
				if !state.update(&taker, price, amount) {
					return Err(Error::OrderStateCheckFailed)
				}
				state
			},
		};

		trie.insert(maker.id.as_ref(), &maker_order_state.encode())?;
		trie.insert(taker.id.as_ref(), &taker_order_state.encode())?;

		// Update balances
		let (maker_asset, maker_credit) = trade.credit(true);
		add_balance(&mut trie, maker_asset, maker_credit)?;
		let (maker_asset, maker_debit) = trade.debit(true);
		sub_balance(&mut trie, maker_asset, maker_debit)?;
		let (taker_asset, taker_credit) = trade.credit(false);
		add_balance(&mut trie, taker_asset, taker_credit)?;
		let (taker_asset, taker_debit) = trade.debit(false);
		sub_balance(&mut trie, taker_asset, taker_debit)?;
		// Commit the state changes in trie
		trie.commit();
		Ok(())
	}

	pub fn process_withdraw(&mut self, withdraw: WithdrawalRequest) -> Result<(), Error> {
		let mut trie =
			TrieDBMutBuilder::from_existing(&mut self.memory_db, &mut self.working_state_root)
				.build();

		// Get main account
		let proxies = trie.get(&withdraw.main.encode())?.ok_or(Error::MainAccountNotFound)?;

		let mut account_info = AccountInfo::decode(&mut &proxies[..])?;
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
		// Queue withdrawal
		self.pending_withdrawals.push(withdraw.try_into()?);
		// Commit the trie
		trie.commit();
		Ok(())
	}

	pub fn handle_blk_import(&mut self, num: BlockNumber) -> Result<(), Error> {
		// Get the ingress messsages for this block
		let messages = self
			.client
			.runtime_api()
			.ingress_messages(&BlockId::number(num.saturated_into()))
			.expect("Expecting ingress messages api to be available");

		let mut trie =
			TrieDBMutBuilder::from_existing(&mut self.memory_db, &mut self.working_state_root)
				.build();

		for message in messages {
			match message {
				IngressMessages::RegisterUser(main, proxy) =>
					register_main(&mut trie, main, proxy)?,
				IngressMessages::Deposit(main, asset, amt) => deposit(&mut trie, main, asset, amt)?,
				IngressMessages::AddProxy(main, proxy) => add_proxy(&mut trie, main, proxy)?,
				IngressMessages::RemoveProxy(main, proxy) => remove_proxy(&mut trie, main, proxy)?,
				IngressMessages::LatestSnapshot(
					snapshot_id,
					state_root,
					state_change_id,
					state_hash,
				) =>
					*self.last_snapshot.write() = SnapshotSummary {
						snapshot_id,
						state_root,
						state_change_id,
						state_hash,
						bitflags: vec![],
						withdrawals: vec![],
						aggregate_signature: None,
					},
				_ => {},
			}
		}

		// 3. Execute RegisterMain, AddProxy, RemoveProxy, Deposit messages, LatestSnapshot
		todo!()
	}

	pub fn get_validator_key(&self, active_set: &Vec<AuthorityId>) -> Result<Public, Error> {
		let available_bls_keys: Vec<Public> = bls_primitives::crypto::bls_ext::all();
		// Get the first available key in the validator set.
		let mut validator_key = None;
		for key in available_bls_keys {
			if active_set.contains(&key.into()) {
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
		let next_snapshot_id = self.last_snapshot.read().snapshot_id + 1;
		let mut summary = self.store_snapshot(stid, next_snapshot_id)?;
		let active_set = self
			.client
			.runtime_api()
			.validator_set(&BlockId::number(self.last_finalized_block.saturated_into()))?
			.validators;
		let signing_key = self.get_validator_key(&active_set)?;
		let signature =
			match bls_primitives::crypto::bls_ext::sign(&signing_key, &summary.sign_data()) {
				Some(sig) => sig,
				None => {
					error!(target:"orderbook","📒 Failed to sign snapshot, not able to sign with validator key.");
					return Err(Error::SnapshotSigningFailed)
				},
			};

		summary.aggregate_signature = Some(signature);
		let bit_index = active_set.iter().position(|v| v == &signing_key.into()).unwrap();
		set_bit_field(&mut summary.bitflags, bit_index as u16);
		// send it to runtime
		if let Err(_) = self
			.client
			.runtime_api()
			.submit_snapshot(&BlockId::number(self.last_finalized_block.into()), summary)
			.expect("Something went wrong with the submit_snapshot runtime api; qed.")
		{
			error!(target:"orderbook","📒 Failed to submit snapshot to runtime");
			return Err(Error::FailedToSubmitSnapshotToRuntime)
		}
		Ok(())
	}

	pub fn handle_action(&mut self, action: ObMessage) -> Result<(), Error> {
		match action.action {
			UserActions::Trade(trade) => self.process_trade(trade)?,
			UserActions::Withdraw(withdraw) => self.process_withdraw(withdraw)?,
			UserActions::BlockImport(num) => self.handle_blk_import(num)?,
			UserActions::Snapshot => self.snapshot(action.stid)?,
		}
		Ok(())
	}

	// Checks if we need to sync the orderbook state before processing the messages.
	pub async fn check_state_sync(&mut self) -> Result<(), Error> {
		// Read latest snapshot from finalizized state
		let summary = self
			.client
			.runtime_api()
			.get_latest_snapshot(&BlockId::number(self.client.info().finalized_number))
			.expect("Something went wrong with the get_latest_snapshot runtime api; qed.");

		self.working_state_root = summary.state_root.as_fixed_bytes().clone();

		// We need to sync only if we are need to update state
		if self.last_snapshot.read().state_change_id < summary.state_change_id {
			// Try to load it from our local DB if not download it from Orderbook operator
			if let Err(_err) = self.load_snapshot(&summary) {
				info!(target: "orderbook", "📒 Orderbook state data not found locally for stid: {:?}",summary.state_change_id);
				self.download_snapshot_from_operator(&summary).await?;
			}
			// X->Y sync: Ask peers to send the missed stid
			if !self.known_messages.is_empty() {
				// Collect all known stids
				let mut known_stids = self.known_messages.keys().collect::<Vec<&u64>>();
				known_stids.sort_unstable(); // unstable is fine since we know stids are unique
							 // if the next best known stid is not available then ask others
				if *known_stids[0] != self.last_snapshot.read().state_change_id.saturating_add(1) {
					// Ask other peers to send us the requests stids.
					let import_request = StidImportRequest {
						from: self.last_snapshot.read().state_change_id,
						to: *known_stids[0],
					};
					let data = import_request.encode();
					for peer in &self.gossip_validator.peers {
						self.send_request_to_peer(
							peer,
							STID_IMPORT_REQUEST.to_string(),
							data.clone(),
						);
					}
				} else {
					info!(target: "orderbook", "📒 sync request not required, we know the next stid");
				}
			} else {
				info!(target: "orderbook", "📒 No new messages known after stid: {:?}",self.last_snapshot.read().state_change_id);
			}
		} else {
			info!(target: "orderbook", "📒 Sync is not required latest stid: {:?}, last_snapshot_stid: {:?}",self.last_snapshot.read().state_change_id, summary.state_change_id);
		}
		Ok(())
	}

	pub fn send_request_to_peer(&self, peer: &PeerId, protocol: String, data: Vec<u8>) {
		self.network.write_notification(*peer, Cow::from(protocol.clone()), data);
	}

	pub async fn download_snapshot_from_operator(
		&mut self,
		summary: &SnapshotSummary,
	) -> Result<(), Error> {
		let active_set = self
			.client
			.runtime_api()
			.validator_set(&BlockId::number(self.last_finalized_block.saturated_into()))?
			.validators;
		match self.get_validator_key(&active_set) {
			Err(_) => return Err(Error::Fullnode),
			Ok(signing_key) => {
				let signature: Signature = match bls_primitives::crypto::bls_ext::sign(
					&signing_key,
					&summary.state_change_id.encode(),
				) {
					Some(sig) => sig,
					None => return Err(Error::BLSSigningFailed),
				};
				// Request for presigned url with signature and stid
				let client = reqwest::Client::new();
				let request = client
					.get(format!("https://snapshots.polkadex.trade/{}", summary.state_change_id))
					.header("Signature", hex::encode(signature.0))
					.build()?;
				info!(target:"orderbook","📒 Requesting presigned url for stid: {:?} from operator",summary.state_change_id);
				let presigned_url = client.execute(request).await?.text().await?;
				info!(target:"orderbook","📒 Downloading snapshot for stid: {:?} from operator",summary.state_change_id);
				// Download data using the presigned url
				let request = client.get(presigned_url).build()?;

				let data = client.execute(request).await?.bytes().await?.to_vec();
				info!(target:"orderbook","📒 Checking hash of downloaded snapshot for stid: {:?}",summary.state_change_id);
				let computed_hash = H256::from(sp_core::blake2_256(&data));
				if computed_hash != summary.state_hash {
					warn!(target:"orderbook","📒 orderbook state hash mismatch: computed: {:?}, expected:
                {:?}",computed_hash,summary.state_hash);
					return Err(Error::StateHashMisMatch)
				}
				self.load_state_from_data(&data, summary)?;
			},
		}

		Ok(())
	}

	pub fn load_state_from_data(
		&mut self,
		data: &Vec<u8>,
		summary: &SnapshotSummary,
	) -> Result<(), Error> {
		match serde_json::from_slice::<HashMap<[u8; 32], (Vec<u8>, i32)>>(data) {
			Ok(data) => {
				self.memory_db.load_from(data);
				let mut summary_clone = summary.clone();
				*self.last_snapshot.write() = summary_clone;
			},
			Err(err) =>
				return Err(Error::Backend(format!("Error decoding snapshot data: {:?}", err))),
		}
		Ok(())
	}

	pub async fn process_new_user_action(&mut self, action: &ObMessage) -> Result<(), Error> {
		// Cache the message
		self.known_messages.insert(action.stid, action.clone());
		if self.sync_oracle.is_major_syncing() {
			info!(target: "orderbook", "📒 Ob message cached for sync to complete: stid: {:?}",action.stid);
			return Ok(())
		}
		self.check_state_sync().await?;
		self.check_stid_gap_fill().await?;
		Ok(())
	}

	pub fn store_snapshot(
		&mut self,
		state_change_id: u64,
		snapshot_id: u64,
	) -> Result<SnapshotSummary, Error> {
		if let Some(mut offchain_storage) = self.backend.offchain_storage() {
			return match serde_json::to_vec(self.memory_db.data()) {
				Ok(data) => {
					offchain_storage.set(
						b"OrderbookSnapshotState",
						&snapshot_id.to_le_bytes(),
						&data,
					);
					let state_hash = H256::from(sp_core::blake2_256(&data));
					let withdrawals = self.pending_withdrawals.clone();
					self.pending_withdrawals.clear();
					Ok(SnapshotSummary {
						snapshot_id,
						state_root: self.working_state_root.into(),
						state_change_id,
						state_hash,
						bitflags: vec![],
						withdrawals,
						aggregate_signature: None,
					})
				},
				Err(err) => Err(Error::Backend(format!("Error serializing the data: {:?}", err))),
			}
		}
		return Err(Error::Backend("Offchain Storage not Found".parse().unwrap()))
	}

	pub fn load_snapshot(&mut self, summary: &SnapshotSummary) -> Result<(), Error> {
		if let Some(offchain_storage) = self.backend.offchain_storage() {
			if let Some(data) =
				offchain_storage.get(b"OrderbookSnapshotState", &summary.snapshot_id.to_le_bytes())
			{
				let computed_hash = H256::from(sp_core::blake2_256(&data));
				if computed_hash != summary.state_hash {
					warn!(target:"orderbook","📒 orderbook state hash mismatch: computed: {:?}, expected: {:?}",computed_hash,summary.state_hash);
					return Err(Error::StateHashMisMatch)
				}
				self.load_state_from_data(&data, summary)?;
			}
		}
		Ok(())
	}

	// Checks if we have all stids to drive the state and then drive it.
	pub async fn check_stid_gap_fill(&mut self) -> Result<(), Error> {
		let mut last_snapshot = self.last_snapshot.read().state_change_id.saturating_add(1);

		while let Some(action) = self.known_messages.remove(&last_snapshot) {
			self.handle_action(action)?;
			last_snapshot = last_snapshot.saturating_add(1);
		}
		// We need to sub 1 since that last processed is one stid less than the not available
		// when while loop is broken
		self.last_snapshot.write().state_change_id = last_snapshot.saturating_sub(1);
		Ok(())
	}

	pub async fn handle_network_event(&mut self, event: &Event) -> Result<(), Error> {
		match event {
			Event::NotificationsReceived { remote, messages } => {
				for (protocol, data) in messages {
					if protocol == STID_IMPORT_REQUEST {
						match StidImportRequest::decode(&mut &data[..]) {
							Ok(request) => {
								let mut response = StidImportResponse::default();
								for stid in request.from..=request.to {
									if let Some(msg) = self.known_messages.get(&stid) {
										response.messages.push(msg.clone());
									}
								}
								if !response.messages.is_empty() {
									self.send_request_to_peer(
										remote,
										STID_IMPORT_RESPONSE.to_string(),
										response.encode(),
									);
								}
							},
							Err(err) => {
								// TODO: reduce reputation for this peer and eventually
								// disconnect if this peer goes below threshold
								error!(target:"orderbook","stid import request cannot be decoded: {:?}",err)
							},
						}
					} else if protocol == STID_IMPORT_RESPONSE {
						match StidImportResponse::decode(&mut &data[..]) {
							Ok(response) => {
								for message in response.messages {
									// TODO: DO signature checks here and handle reputation
									self.known_messages.entry(message.stid).or_insert(message);
								}

								self.check_stid_gap_fill().await?
							},
							Err(err) => {
								// TODO: reduce reputation for this peer and eventually
								// disconnect if this peer goes below threshold
								error!(target:"orderbook","stid import request cannot be decoded: {:?}",err)
							},
						}
					} else {
						warn!(target:"orderbook","Ignoring network event for protocol: {:?}",protocol)
					}
				}
			},
			_ => {},
		}

		Ok(())
	}

	async fn handle_finality_notification(
		&mut self,
		notification: &FinalityNotification<B>,
	) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Finality notification for blk: {:?}", notification.header.number());
		let header = &notification.header;
		self.last_finalized_block = (*header.number()).saturated_into();
		// TODO: Detect if our snapshot was accepted if not then change our state
		// to the accepted snapshot
		Ok(())
	}

	/// Wait for Orderbook runtime pallet to be available.
	async fn wait_for_runtime_pallet(&mut self) {
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		while let Some(notif) = finality_stream.next().await {
			let at = BlockId::hash(notif.header.hash());
			if self.client.runtime_api().validator_set(&at).ok().is_some() {
				break
			} else {
				debug!(target: "orderbook", "📒 Waiting for orderbook pallet to become available...");
			}
		}
	}

	/// Main loop for Orderbook worker.
	///
	/// Wait for Orderbook runtime pallet to be available, then start the main async loop
	/// which is driven by gossiped user actions.
	pub(crate) async fn run(mut self) {
		info!(target: "orderbook", "📒 Orderbook worker started");
		self.wait_for_runtime_pallet().await;
		let mut gossip_messages = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					trace!(target: "orderbook", "📒 Got gossip message: {:?}", notification);

					ObMessage::decode(&mut &notification.message[..]).ok()
				})
				.fuse(),
		);
		// network events stream
		let mut notification_events_stream = self.network.event_stream("orderbook").fuse();
		// finality events stream
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		loop {
			let mut gossip_engine = &mut self.gossip_engine;
			futures::select_biased! {
				gossip = gossip_messages.next() => {
					if let Some(message) = gossip {
						// Gossip messages have already been verified to be valid by the gossip validator.
						if let Err(err) = self.process_new_user_action(&message).await {
							debug!(target: "orderbook", "📒 {}", err);
						}
					} else {
						return;
					}
				},
				message = self.message_sender_link.next() => {
					if let Some(message) = message {
						if let Err(err) = self.process_new_user_action(&message).await {
							debug!(target: "orderbook", "📒 {}", err);
						}
					}else{
						return;
					}
				},
				notification = notification_events_stream.next() => {

					if let Some(notification) = notification {
					if let Err(err) = self.handle_network_event(&notification).await {
							debug!(target: "orderbook", "📒 {}", err);
					}
					}else {
						error!(target:"orderbook","None notification recvd");
						return
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

pub fn register_main(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	proxy: AccountId,
) -> Result<(), Error> {
	if trie.contains(&main.encode())? {
		return Err(Error::MainAlreadyRegistered)
	}
	let account_info = AccountInfo { proxies: vec![proxy] };
	trie.insert(&main.encode(), &account_info.encode())?;
	Ok(())
}

pub fn add_proxy(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	proxy: AccountId,
) -> Result<(), Error> {
	match trie.get(&main.encode())? {
		Some(data) => {
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

pub fn remove_proxy(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	proxy: AccountId,
) -> Result<(), Error> {
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
				// its a no-op if proxy not found
			}
		},
		None => return Err(Error::MainAccountNotFound),
	}
	Ok(())
}

pub fn deposit(
	trie: &mut TrieDBMut<ExtensionLayout>,
	main: AccountId,
	asset: AssetId,
	amount: Decimal,
) -> Result<(), Error> {
	if !trie.contains(&main.encode())? {
		return Err(Error::MainAccountNotFound)
	}
	let account_asset = AccountAsset { main, asset };
	match trie.get(&account_asset.encode())? {
		Some(data) => {
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
