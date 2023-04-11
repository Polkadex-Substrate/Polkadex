use std::{
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	sync::Arc,
	time::Duration,
};

use chrono::Utc;
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use log::{debug, error, info, trace, warn};
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
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
use tokio::time::Interval;

use bls_primitives::Public;
use thea_primitives::{
	crypto::AuthorityId,
	types::{prepare_bitmap, Message},
	Network, TheaApi, ValidatorSet, NATIVE_NETWORK,
};

use crate::{
	connector::{parachain::ParachainClient, traits::ForeignConnector},
	error::Error,
	gossip::{topic, GossipValidator},
	metric_add, metric_inc, metric_set,
	metrics::Metrics,
	traits::ForeignConnector,
	types::GossipMessage,
	Client,
};

pub(crate) struct WorkerParams<B: Block, BE, C, SO, N, R> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub sync_oracle: SO,
	pub metrics: Option<Metrics>,
	pub is_validator: bool,
	pub message_sender_link: UnboundedReceiver<Network>,
	/// Gossip network
	pub network: N,
	/// Chain specific Ob protocol name. See [`thea_protocol_name::standard_name`].
	pub protocol_name: sc_network::ProtocolName,
	pub _marker: PhantomData<B>,
}

/// A Orderbook worker plays the Orderbook protocol
pub(crate) struct ObWorker<B: Block, BE, C, SO, N, R> {
	// utilities
	client: Arc<C>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	sync_oracle: SO,
	metrics: Option<Metrics>,
	is_validator: bool,
	network: Arc<N>,
	thea_network: Option<Network>,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	message_sender_link: UnboundedReceiver<Network>,
	// Payload to gossip message mapping
	message_cache: BTreeMap<Message, GossipMessage>,
	foreign_chain: Arc<dyn ForeignConnector>,
}

impl<B, BE, C, SO, N, R> ObWorker<B, BE, C, SO, N, R>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
	SO: Send + Sync + Clone + 'static + SyncOracle,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
{
	/// Return a new BEEFY worker instance.
	///
	/// Note that a BEEFY worker is only fully functional if a corresponding
	/// BEEFY pallet has been deployed on-chain.
	///
	/// The BEEFY pallet is needed in order to keep track of the BEEFY authority set.
	pub(crate) async fn new(
		worker_params: WorkerParams<B, BE, C, SO, N, R>,
	) -> Result<Self, Error> {
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
		} = worker_params;

		let gossip_validator = Arc::new(GossipValidator::new());
		let gossip_engine =
			GossipEngine::new(network.clone(), protocol_name, gossip_validator.clone(), None);

		let foreign_connector = ParachainClient::connect().await?;

		Ok(ObWorker {
			client,
			backend,
			runtime,
			metrics,
			sync_oracle,
			is_validator,
			network: Arc::new(network),
			thea_network: None,
			gossip_engine,
			gossip_validator,
			message_sender_link,
			message_cache: Default::default(),
			foreign_chain: Arc::new(foreign_connector),
		})
	}

	pub async fn update_network_pref(&mut self, network: Network) -> Result<(), Error> {
		self.thea_network = Some(network);
		// TODO: Store it in local storage too.
		Ok(())
	}

	pub fn get_validator_key(&self, active_set: &Vec<AuthorityId>) -> Result<Public, Error> {
		let available_bls_keys: Vec<Public> = bls_primitives::crypto::bls_ext::all();
		info!(target:"orderbook","📒 Avaialble BLS keys: {:?}",available_bls_keys);
		info!(target:"orderbook","📒 Active BLS keys: {:?}",active_set);
		// Get the first available key in the validator set.
		let mut validator_key = None;
		for key in available_bls_keys {
			if active_set.contains(&thea_primitives::crypto::AuthorityId::from(key)) {
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

	pub async fn process_gossip_message(
		&mut self,
		message: &GossipMessage,
		remote: Option<PeerId>,
	) -> Result<(), Error> {
		metric_inc!(self, thea_messages_recv);
		metric_add!(self, thea_data_recv, message.encoded_size() as u64);
		match self.message_cache.get(&message.payload) {
			None => self.message_cache.insert(message.payload.clone(), message.clone()),
			Some(message) => {
				// TODO
				// 1. incoming message has more signatories
				// 2. Check if our signature is included or not
				// 3. Aggregate the signature
				// 4. if majority is achieved, send it to foreign/native chain
				if message.payload.network == NATIVE_NETWORK {
					self.runtime.runtime_api().incoming_message(
						message.payload.clone(),
						message.bitmap.clone(),
						message.aggregate_signature,
					)??;
				} else {
					self.foreign_chain.send_transaction(message.clone()).await;
				}
				//  5. else, update the local state
			},
		}

		Ok(())
	}

	pub(crate) async fn handle_finality_notification(
		&mut self,
		notification: &FinalityNotification<B>,
	) -> Result<(), Error> {
		info!(target: "orderbook", "📒 Finality notification for blk: {:?}", notification.header.number());
		let header = &notification.header;
		let at = BlockId::hash(header.hash());

		// Proceed only if we are a validator
		if !self.is_validator {
			return Ok(())
		}

		if self.thea_network.is_none() {
			log::error!(target:"thea","Thea network is not configured for this validator, please use the local rpc");
			return Err(Error::NetworkNotConfigured)
		}
		let network = self.thea_network.unwrap();

		if let Some(message) = self.runtime.runtime_api().outgoing_messages(
			&at,
			header.number().saturated_into(),
			network,
		)? {
			self.sign_and_submit_message(message).await?;
		}
		Ok(())
	}

	pub async fn sign_and_submit_message(&mut self, message: Message) -> Result<(), Error> {
		// Check if we are part of active network
		let active = self.runtime.runtime_api().validator_set(&at, network)?;

		let signing_key = self.get_validator_key(&active.validators)?;
		let signature = match bls_primitives::crypto::sign(&signing_key, &message.encode()) {
			Some(sig) => sig,
			None => {
				error!(target:"orderbook","📒 Failed to thea message, not able to sign with validator key.");
				return Err(Error::SigningFailed)
			},
		};

		let bitmap = prepare_bitmap(&active.validators, &signing_key.into());

		// Gossip this message to every one
		let gossip_message =
			GossipMessage { payload: message.clone(), bitmap, aggregate_signature: signature };
		self.gossip_engine.gossip_message(topic(), gossip_message.encode(), true);
		self.process_gossip_message(&gossip_message, None)
	}

	/// Wait for Orderbook runtime pallet to be available.
	pub(crate) async fn wait_for_runtime_pallet(&mut self) {
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		while let Some(notif) = finality_stream.next().await {
			let at = BlockId::hash(notif.header.hash());
			if self.runtime.runtime_api().validator_set(&at).ok().is_some() {
				break
			} else {
				debug!(target: "orderbook", "📒 Waiting for thea pallet to become available...");
			}
		}
	}

	pub async fn try_process_foreign_chain_events(&mut self) -> Result<(), Error> {
		// TODO: Provide the block number to use
		// Get the next block events from foreign chain as Message
		let message = self.foreign_chain.read_events().await?;
		self.sign_and_submit_message(message).await
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

		// Interval timer to read foreign chain events
		let interval = tokio::time::interval(self.foreign_chain.block_duration());

		loop {
			let mut gossip_engine = &mut self.gossip_engine;
			futures::select_biased! {
				gossip = gossip_messages.next() => {
					if let Some((message,sender)) = gossip {
						// Gossip messages have already been verified to be valid by the gossip validator.
						if let Err(err) = self.process_gossip_message(&message,sender).await {
							debug!(target: "orderbook", "📒 {}", err);
						}
					} else {
						return;
					}
				},
				message = self.message_sender_link.next() => {
					if let Some(message) = message {
						if let Err(err) = self.update_network_pref(message).await {
							debug!(target: "orderbook", "📒 {}", err);
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
				_ = interval.tick() => {
					if let Err(err) = self.try_process_foreign_chain_events().await {
							error!(target: "orderbook", "📒 Error during finalized block import{}", err);
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
