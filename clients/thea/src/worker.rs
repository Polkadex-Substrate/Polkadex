use std::{collections::BTreeMap, marker::PhantomData, ops::AddAssign, sync::Arc, time::Duration};

use futures::StreamExt;
use log::{debug, error, info};
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
use polkadex_primitives::utils::{prepare_bitmap, return_set_bits, set_bit_field};
use sc_client_api::{Backend, FinalityNotification};
use sc_keystore::LocalKeystore;
use sc_network::PeerId;
use sc_network_gossip::{GossipEngine, Network as GossipNetwork};
use sp_api::ProvideRuntimeApi;
use sp_arithmetic::traits::SaturatedConversion;
use sp_consensus::SyncOracle;
use sp_runtime::{
	generic::BlockId,
	traits::{Block, Header, Zero},
};
use thea_primitives::{types::Message, AuthorityIndex, Network, TheaApi, NATIVE_NETWORK};

use crate::{
	connector::traits::ForeignConnector,
	error::Error,
	gossip::{topic, GossipValidator},
	keystore::TheaKeyStore,
	metric_add, metric_inc,
	metrics::Metrics,
	types::GossipMessage,
	Client,
};

pub(crate) struct WorkerParams<B: Block, BE, C, SO, N, R, FC: ForeignConnector + ?Sized> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub sync_oracle: SO,
	pub metrics: Option<Metrics>,
	pub is_validator: bool,
	/// Gossip network
	pub network: N,
	/// Chain specific Ob protocol name. See [`thea_protocol_name::standard_name`].
	pub protocol_name: sc_network::ProtocolName,
	pub _marker: PhantomData<B>,
	pub foreign_chain: Arc<FC>,
	pub(crate) keystore: Option<Arc<LocalKeystore>>,
}

/// A Orderbook worker plays the Orderbook protocol
pub(crate) struct ObWorker<B: Block, BE, C, SO, N, R, FC: ForeignConnector + ?Sized> {
	// utilities
	pub(crate) client: Arc<C>,
	_backend: Arc<BE>,
	runtime: Arc<R>,
	sync_oracle: SO,
	metrics: Option<Metrics>,
	is_validator: bool,
	_network: Arc<N>,
	keystore: TheaKeyStore,
	thea_network: Option<Network>,
	gossip_engine: GossipEngine<B>,
	_gossip_validator: Arc<GossipValidator<B>>,
	// Payload to gossip message mapping
	pub(crate) message_cache: Arc<RwLock<BTreeMap<Message, GossipMessage>>>,
	last_foreign_nonce_processed: Arc<RwLock<u64>>,
	last_native_nonce_processed: Arc<RwLock<u64>>,
	foreign_chain: Arc<FC>,
	last_finalized_blk: BlockId<B>,
}

impl<B, BE, C, SO, N, R, FC> ObWorker<B, BE, C, SO, N, R, FC>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
	SO: Send + Sync + Clone + 'static + SyncOracle,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	FC: ForeignConnector + ?Sized,
{
	/// Return a new BEEFY worker instance.
	///
	/// Note that a BEEFY worker is only fully functional if a corresponding
	/// BEEFY pallet has been deployed on-chain.
	///
	/// The BEEFY pallet is needed in order to keep track of the BEEFY authority set.
	pub(crate) async fn new(worker_params: WorkerParams<B, BE, C, SO, N, R, FC>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			foreign_chain,
			keystore,
			sync_oracle,
			metrics,
			is_validator,
			network,
			protocol_name,
			_marker,
		} = worker_params;

		let message_cache = Arc::new(RwLock::new(BTreeMap::new()));
		let foreign_nonce = Arc::new(RwLock::new(0));
		let native_nonce = Arc::new(RwLock::new(0));
		let gossip_validator = Arc::new(GossipValidator::new(
			message_cache.clone(),
			foreign_nonce.clone(),
			native_nonce.clone(),
		));
		let gossip_engine =
			GossipEngine::new(network.clone(), protocol_name, gossip_validator.clone(), None);

		ObWorker {
			client,
			_backend: backend,
			runtime,
			metrics,
			sync_oracle,
			is_validator,
			_network: Arc::new(network),
			keystore: TheaKeyStore::new(keystore),
			thea_network: None,
			gossip_engine,
			_gossip_validator: gossip_validator,
			message_cache,
			last_foreign_nonce_processed: foreign_nonce,
			last_native_nonce_processed: native_nonce,
			foreign_chain,
			last_finalized_blk: BlockId::number(Zero::zero()),
		}
	}

	pub fn sign_message(&mut self, message: Message) -> Result<GossipMessage, Error> {
		let network = self.thea_network.expect("Expected the network to be defined here.");
		info!(target:"thea", "Serving network: {:?}", network);
		let active = self
			.runtime
			.runtime_api()
			.validator_set(&self.last_finalized_blk, network)?
			.ok_or(Error::ValidatorSetNotInitialized(network))?;

		let signing_key = self.keystore.get_local_key(&active.validators)?;
		let signature = self.keystore.sign(&signing_key, &message.encode())?;
		info!(target:"thea", "Signature generated for thea");
		let bit_index = active.validators.iter().position(|x| *x == signing_key).unwrap();

		let bitmap: Vec<u128> = prepare_bitmap(&vec![bit_index], active.validators.len()).unwrap();

		Ok(GossipMessage { payload: message, bitmap, aggregate_signature: signature.into() })
	}

	pub async fn check_message(&mut self, message: &GossipMessage) -> Result<bool, Error> {
		// TODO: Do signature check here.
		// Based on network use the corresponding api to check if the message if valid or not.
		if message.payload.network != NATIVE_NETWORK {
			self.foreign_chain.check_message(&message.payload).await
		} else {
			let finalized_blk = self.last_finalized_blk;
			let network = self.thea_network.unwrap();
			let result = self
				.runtime
				.runtime_api()
				.outgoing_messages(&finalized_blk, network, message.payload.nonce)?
				.ok_or(Error::ErrorReadingTheaMessage)?;

			Ok(result == message.payload)
		}
	}

	pub async fn process_gossip_message(
		&mut self,
		incoming_message: &mut GossipMessage,
		_: Option<PeerId>,
	) -> Result<(), Error> {
		if !self.is_validator {
			return Ok(())
		}
		metric_inc!(self, thea_messages_recv);
		metric_add!(self, thea_data_recv, incoming_message.encoded_size() as u64);
		let local_index = self.get_local_auth_index()?;
		let option = self.message_cache.read().get(&incoming_message.payload).cloned();
		// Check incoming message in our cache.
		match option {
			None => {
				// Check if the incoming message is valid based on our local node
				match self.check_message(incoming_message).await? {
					false => {
						error!(target:"thea", "Message check failed");
						// TODO: We will do offence handler later, simply ignore now
						return Ok(())
					},
					true => {
						// Sign the message
						let gossip_message = self.sign_message(incoming_message.payload.clone())?;

						// Aggregate the signature and store it.
						incoming_message.aggregate_signature = incoming_message
							.aggregate_signature
							.add_signature(&gossip_message.aggregate_signature)?;
						// Set the bit based on our local index
						set_bit_field(&mut incoming_message.bitmap, local_index.saturated_into());
						info!(target:"thea","Message status: nonce: {:?}, signed: {:?}, threshold: {:?}",
							incoming_message.payload.nonce,
							return_set_bits(&incoming_message.bitmap).len(),
							incoming_message.payload.threshold()
						);
						if return_set_bits(&incoming_message.bitmap).len() >=
							incoming_message.payload.threshold() as usize
						{
							// We got majority on this message
							if incoming_message.payload.network == NATIVE_NETWORK {
								self.foreign_chain
									.send_transaction(incoming_message.clone())
									.await?;
							} else {
								self.runtime.runtime_api().incoming_message(
									&self.last_finalized_blk,
									incoming_message.payload.clone(),
									incoming_message.bitmap.clone(),
									incoming_message.aggregate_signature.into(),
								)??;
							}
							self.message_cache.write().remove(&incoming_message.payload);
						} else {
							// Cache it.
							self.message_cache
								.write()
								.insert(incoming_message.payload.clone(), incoming_message.clone());
						}
					},
				}
			},
			Some(message) => {
				// 1. incoming message has more signatories
				let signed_auth_indexes = return_set_bits(&incoming_message.bitmap);
				// 2. Check if our signature is included or not
				let did_we_sign_incoming_message =
					signed_auth_indexes.contains(&local_index.saturated_into());

				// There are two cases here,
				if !did_we_sign_incoming_message {
					// Let's add our signature to it
					let gossip_message = self.sign_message(incoming_message.payload.clone())?;

					// Aggregate the signature and store it.
					incoming_message.aggregate_signature = incoming_message
						.aggregate_signature
						.add_signature(&gossip_message.aggregate_signature)?;
					// Set the bit based on our local index
					set_bit_field(&mut incoming_message.bitmap, local_index.saturated_into());
					info!(target:"thea","Message status: nonce: {:?}, signed: {:?}, threshold: {:?}",
						incoming_message.payload.nonce,
						return_set_bits(&incoming_message.bitmap).len(),
						incoming_message.payload.threshold()
					);
					if return_set_bits(&incoming_message.bitmap).len() >=
						incoming_message.payload.threshold() as usize
					{
						info!(target:"thea","Got majority on message: nonce: {:?}, network: {:?}", message.payload.nonce, message.payload.network);
						// We got majority on this message
						if incoming_message.payload.network == NATIVE_NETWORK {
							self.foreign_chain.send_transaction(incoming_message.clone()).await?;
						} else {
							self.runtime.runtime_api().incoming_message(
								&self.last_finalized_blk,
								incoming_message.payload.clone(),
								incoming_message.bitmap.clone(),
								incoming_message.aggregate_signature.into(),
							)??;
						}
						self.message_cache.write().remove(&incoming_message.payload);
					} else {
						// Cache it.
						self.message_cache
							.write()
							.insert(incoming_message.payload.clone(), incoming_message.clone());
						// TODO: Send it back to network.
					}
				}
			},
		}

		Ok(())
	}

	pub(crate) async fn handle_finality_notification(
		&mut self,
		notification: &FinalityNotification<B>,
	) -> Result<(), Error> {
		info!(target: "thea", "📒 Finality notification for blk: {:?}", notification.header.number());
		let header = &notification.header;
		let at = BlockId::hash(header.hash());
		self.last_finalized_blk = at;
		// Proceed only if we are a validator
		if !self.is_validator {
			return Ok(())
		}

		if self.thea_network.is_none() {
			let active = self
				.runtime
				.runtime_api()
				.full_validator_set(&at)?
				.ok_or(Error::NoValidatorsFound)?;
			let signing_key = self.keystore.get_local_key(active.validators())?;
			let network = self.runtime.runtime_api().network(&at, signing_key)?;

			if network.is_none() {
				log::error!(target:"thea","Thea network is not configured for this validator, please use the local rpc");
				return Err(Error::NetworkNotConfigured)
			} else {
				self.thea_network = network;
			}
		}
		let network = self.thea_network.unwrap();

		// Get the last processed foreign nonce from native
		let best_incoming_nonce: u64 = self
			.runtime
			.runtime_api()
			.get_last_processed_nonce(&self.last_finalized_blk, network)?;

		*self.last_foreign_nonce_processed.write() = best_incoming_nonce;

		let last_nonce = self.foreign_chain.last_processed_nonce_from_native().await?;

		let next_nonce_to_process = last_nonce.saturating_add(1);

		let message =
			self.runtime
				.runtime_api()
				.outgoing_messages(&at, network, next_nonce_to_process)?;

		if let Some(message) = message {
			info!(target:"thea", "Processing new message from native chain: nonce: {:?}, to_network: {:?}",message.nonce, message.network);
			self.sign_and_submit_message(message)?;
		}

		Ok(())
	}

	pub fn get_local_auth_index(&self) -> Result<AuthorityIndex, Error> {
		let network = self.thea_network.expect("Expected the thea network to be initialized");
		let active = self
			.runtime
			.runtime_api()
			.validator_set(&self.last_finalized_blk, network)?
			.ok_or(Error::ValidatorSetNotInitialized(network))?;

		let signing_key = self.keystore.get_local_key(&active.validators)?;

		// Unwrap is fine since we already know we are in that list
		let index = active.validators.iter().position(|x| x == &signing_key).unwrap();
		Ok(index.saturated_into())
	}

	pub fn sign_and_submit_message(&mut self, message: Message) -> Result<(), Error> {
		let gossip_message = self.sign_message(message.clone())?;
		self.gossip_engine.gossip_message(topic::<B>(), gossip_message.encode(), true);
		self.message_cache.write().insert(message, gossip_message);
		Ok(())
	}

	/// Wait for Orderbook runtime pallet to be available.
	pub(crate) async fn wait_for_runtime_pallet(&mut self) {
		let mut finality_stream = self.client.finality_notification_stream().fuse();
		while let Some(notif) = finality_stream.next().await {
			let at = BlockId::hash(notif.header.hash());
			if self.runtime.runtime_api().validator_set(&at, 0).ok().is_some() {
				break
			} else {
				debug!(target: "orderbook", "📒 Waiting for thea pallet to become available...");
			}
		}
	}

	pub async fn try_process_foreign_chain_events(&mut self) -> Result<(), Error> {
		// Proceed only if we are a validator
		if !self.is_validator {
			return Ok(())
		}
		match self.thea_network.as_ref() {
			None => {
				log::error!(target:"thea", "Thea network not set on this validator!");
				return Ok(())
			},
			Some(network) => {
				// Get the next block events from foreign chain as Message
				let mut best_outgoing_nonce: u64 = self
					.runtime
					.runtime_api()
					.get_last_processed_nonce(&self.last_finalized_blk, *network)?;

				// Get the last processed native nonce from foreign
				let last_nonce = self.foreign_chain.last_processed_nonce_from_native().await?;

				*self.last_native_nonce_processed.write() = last_nonce;

				info!(target:"thea","Checking new messages on network: {network:?}, last nonce from native: {best_outgoing_nonce:?}");
				best_outgoing_nonce.add_assign(1);

				// Check if next best message is available for processing
				match self.foreign_chain.read_events(best_outgoing_nonce).await? {
					None => {},
					Some(message) => {
						// Don't do anything if we already know about the message
						// It means Thea is already processing it.
						if !self.message_cache.read().contains_key(&message) {
							info!(target:"thea", "Found new message for processing.. network:{:?} nonce: {:?}",message.network, message.nonce);
							self.sign_and_submit_message(message)?
						}
					},
				}
			},
		}
		Ok(())
	}

	/// Main loop for Orderbook worker.
	///
	/// Wait for Orderbook runtime pallet to be available, then start the main async loop
	/// which is driven by gossiped user actions.
	pub(crate) async fn run(mut self) {
		info!(target: "thea", "Thea worker started");
		self.wait_for_runtime_pallet().await;

		// Wait for blockchain sync to complete
		while self.sync_oracle.is_major_syncing() {
			info!(target: "thea", "Thea is not started waiting for blockhchain to sync completely");
			tokio::time::sleep(Duration::from_secs(12)).await;
		}

		// TODO: Check if validator has provided the network pref, elser, panic or log.

		info!(target:"thea"," Starting event streams...");
		let mut gossip_messages = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					info!(target: "thea", "📒 Got gossip message : {:?}", notification);
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
		// create a stream from the interval
		let mut interval_stream = tokio_stream::wrappers::IntervalStream::new(interval).fuse();

		loop {
			let mut gossip_engine = &mut self.gossip_engine;
			futures::select_biased! {
				_ = gossip_engine => {
					error!(target: "orderbook", "📒 Gossip engine has terminated.");
					return;
				}
				gossip = gossip_messages.next() => {
					if let Some((mut message,sender)) = gossip {
						info!(target:"thea","Got new message via gossip : nonce: {:?}, signed: {:?}, threshold: {:?}",
						message.payload.nonce,
						return_set_bits(&message.bitmap).len(),
						message.payload.threshold()
					);
						// Gossip messages have already been verified to be valid by the gossip validator.
						if let Err(err) = self.process_gossip_message(&mut message,sender).await {
							debug!(target: "orderbook", "📒 {:?}", err);
						}
					} else {
						return;
					}
				},
				finality = finality_stream.next() => {
					if let Some(finality) = finality {
						if let Err(err) = self.handle_finality_notification(&finality).await {
							error!(target: "orderbook", "📒 Error during finalized block import{:?}", err);
						}
					}else {
						error!(target:"orderbook","None finality recvd");
						return
					}
				},
				_ = interval_stream.next() => {
					if let Err(err) = self.try_process_foreign_chain_events().await {
							error!(target: "orderbook", "📒 Error fetching foreign chain events {:?}", err);
						}
				},
			}
		}
	}
}
