use crate::{
	error::Error,
	gossip,
	gossip::{topic, GossipValidator},
	metrics::Metrics,
	Client,
};
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use log::{debug, error, info, trace};
use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::{types::ObMessage, ObApi};
use parity_scale_codec::{Codec, Decode, Encode};
use parking_lot::RwLock;
use sc_client_api::Backend;
use sc_network_gossip::GossipEngine;
use sp_api::ProvideRuntimeApi;
use sp_runtime::{generic::BlockId, traits::Block};
use std::{
	collections::{BTreeMap, HashMap},
	marker::PhantomData,
	sync::Arc,
};

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
	// key_store: BeefyKeystore,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	// Last processed state change id
	last_processed_stid: Arc<RwLock<u64>>,
	// Known state ids
	known_messages: BTreeMap<u64, ObMessage>,
	// Links between the block importer, the background voter and the RPC layer.
	// links: BeefyVoterLinks<B>,

	// voter state
	/// Orderbook client metrics.
	metrics: Option<Metrics>,
	message_sender_link: UnboundedReceiver<ObMessage>,
	_marker: PhantomData<N>,
	// In memory store
	memory_db: MemoryDB<CustomBlake2Hasher, HashKey<CustomBlake2Hasher>, Vec<u8>>,
}
use crate::hasher::CustomBlake2Hasher;
use orderbook_primitives::types::UserActions;
use sc_network_gossip::Network as GossipNetwork;
use sp_core::{offchain::OffchainStorage, H256};
use sp_trie::{LayoutV1, TrieDBMut};
use trie_db::{FatDBIterator, FatDBMut, TrieDBIterator, TrieMut};

impl<B, BE, C, SO, N> ObWorker<B, BE, C, SO, N>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE> + ProvideRuntimeApi<B>,
	C::Api: ObApi<B>,
	SO: Send + Sync + Clone + 'static,
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

		let last_processed_stid = Arc::new(RwLock::new(0));
		let gossip_validator = Arc::new(GossipValidator::new(last_processed_stid.clone()));
		let gossip_engine = sc_network_gossip::GossipEngine::new(
			network,
			protocol_name,
			gossip_validator.clone(),
			None,
		);

		let _last_finalized_header = client
			.expect_header(BlockId::number(client.info().finalized_number))
			.expect("latest block always has header available; qed.");

		ObWorker {
			client: client.clone(),
			backend,
			sync_oracle,
			// key_store,
			gossip_engine,
			gossip_validator,
			memory_db: MemoryDB::default(),
			// links,
			// last_processed_state_change_id,
			message_sender_link,
			metrics,
			last_processed_stid,
			_marker: Default::default(),
			known_messages: Default::default(),
		}
	}

	pub fn handle_action(&mut self, action: UserActions) -> Result<(), Error> {
		// TODO: All user logic goes here
		todo!()
	}

	pub fn process_message(&mut self, message: &ObMessage) -> Result<(), Error> {
		if !self.known_messages.contains_key(&message.stid) {
			self.handle_action(message.action.clone())?;
			// We gossip this message to others
			self.gossip_engine.gossip_message(topic::<B>(), message.encode(), true);
			let mut next_to_process = message.stid.saturating_add(1);
			// Check if any other available messages can be processed.
			while let Some(message) = self.known_messages.get(&next_to_process) {
				self.handle_action(message.action.clone())?;
				next_to_process = next_to_process.saturating_add(1);
			}
		}
		Ok(())
	}

	pub fn handle_gossip_message(&mut self, message: &ObMessage) -> Result<(), Error> {
		if self.known_messages.contains_key(&message.stid) {
			return Ok(())
		}
		self.process_message(message)
	}

	pub fn handle_ob_message(&mut self, message: &ObMessage) -> Result<(), Error> {
		if self.known_messages.contains_key(&message.stid) {
			return Ok(())
		}
		self.process_message(message)
	}

	pub fn store_snapshot(&mut self, snapshot_id: u64) -> Result<(), Error> {
		if let Some(mut offchain_storage) = self.backend.offchain_storage() {
			match serde_json::to_vec(self.memory_db.data()) {
				Ok(data) => offchain_storage.set(
					b"OrderbookSnapshotState",
					&snapshot_id.to_le_bytes(),
					&data,
				),
				Err(err) =>
					return Err(Error::Backend(format!("Error serializing the data: {:?}", err))),
			}
		}
		return Err(Error::Backend("Offchain Storage not Found".parse().unwrap()))
	}

	pub fn load_snapshot(&mut self, snapshot_id: u64) -> Result<(), Error> {
		if let Some(offchain_storage) = self.backend.offchain_storage() {
			if let Some(mut data) =
				offchain_storage.get(b"OrderbookSnapshotState", &snapshot_id.to_le_bytes())
			{
				match serde_json::from_slice::<HashMap<H256, (Vec<u8>, i32)>>(&data) {
					Ok(data) => self.memory_db.load_from(data),
					Err(err) =>
						return Err(Error::Backend(format!(
							"Error decoding snapshot data: {:?}",
							err
						))),
				}
			}
		}
		Ok(())
	}

	/// Main loop for Orderbook worker.
	///
	/// Wait for Orderbook runtime pallet to be available, then start the main async loop
	/// which is driven by gossiped user actions.
	pub(crate) async fn run(mut self) {
		info!(target: "orderbook", "📒 Orderbook worker started");
		// self.wait_for_runtime_pallet().await;
		let mut gossip_messages = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					trace!(target: "orderbook", "📒 Got gossip message: {:?}", notification);

					ObMessage::decode(&mut &notification.message[..]).ok()
				})
				.fuse(),
		);

		loop {
			let mut gossip_engine = &mut self.gossip_engine;
			futures::select_biased! {
				gossip = gossip_messages.next() => {
					if let Some(message) = gossip {
						// Gossip messages have already been verified to be valid by the gossip validator.
						if let Err(err) = self.handle_gossip_message(&message) {
							debug!(target: "orderbook", "📒 {}", err);
						}
					} else {
						return;
					}
				},
				message = self.message_sender_link.next() => {
					if let Some(message) = message {
						if let Err(err) = self.handle_ob_message(&message) {
							debug!(target: "orderbook", "📒 {}", err);
						}
					}else{
						return;
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
