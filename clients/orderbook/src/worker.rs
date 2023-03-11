use crate::{
	error::Error,
	gossip::{topic, GossipValidator},
	metrics::Metrics,
	Client,
};
use futures::StreamExt;
use log::{debug, error, info, trace};
use orderbook_primitives::{types::ObMessage, ObApi};
use parity_scale_codec::{Codec, Decode};
use sc_client_api::Backend;
use sc_network_gossip::GossipEngine;
use sp_api::ProvideRuntimeApi;
use sp_runtime::{generic::BlockId, traits::Block};
use std::sync::Arc;

pub(crate) struct WorkerParams<B: Block, BE, C, R, SO> {
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	pub sync_oracle: SO,
	// pub key_store: BeefyKeystore,
	pub gossip_engine: GossipEngine<B>,
	pub gossip_validator: Arc<GossipValidator<B>>,
	// pub links: BeefyVoterLinks<B>,
	pub metrics: Option<Metrics>,
}

/// A Orderbook worker plays the Orderbook protocol
pub(crate) struct ObWorker<B: Block, BE, C, R, SO> {
	// utilities
	client: Arc<C>,
	backend: Arc<BE>,
	runtime: Arc<R>,
	sync_oracle: SO,
	// key_store: BeefyKeystore,
	gossip_engine: GossipEngine<B>,
	gossip_validator: Arc<GossipValidator<B>>,
	// channels
	// Links between the block importer, the background voter and the RPC layer.
	// links: BeefyVoterLinks<B>,

	// voter state
	/// Orderbook client metrics.
	metrics: Option<Metrics>,
}

impl<B, BE, C, R, SO> ObWorker<B, BE, C, R, SO>
where
	B: Block + Codec,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: ObApi<B>,
	SO: Send + Sync + Clone + 'static,
{
	/// Return a new BEEFY worker instance.
	///
	/// Note that a BEEFY worker is only fully functional if a corresponding
	/// BEEFY pallet has been deployed on-chain.
	///
	/// The BEEFY pallet is needed in order to keep track of the BEEFY authority set.
	pub(crate) fn new(worker_params: WorkerParams<B, BE, C, R, SO>) -> Self {
		let WorkerParams {
			client,
			backend,
			runtime,
			// key_store,
			sync_oracle,
			gossip_engine,
			gossip_validator,
			// links,
			metrics,
		} = worker_params;

		let last_finalized_header = client
			.expect_header(BlockId::number(client.info().finalized_number))
			.expect("latest block always has header available; qed.");

		ObWorker {
			client: client.clone(),
			backend,
			runtime,
			sync_oracle,
			// key_store,
			gossip_engine,
			gossip_validator,
			// links,
			// last_processed_state_change_id,
			metrics,
		}
	}

	pub fn handle_gossip_message(
		&mut self,
		message: orderbook_primitives::types::ObMessage,
	) -> Result<(), Error> {
		todo!()
	}

	/// Main loop for Orderbook worker.
	///
	/// Wait for Orderbook runtime pallet to be available, then start the main async loop
	/// which is driven by gossiped user actions.
	pub(crate) async fn run(mut self) {
		info!(target: "orderbook", "🥩 Orderbook worker started");
		// self.wait_for_runtime_pallet().await;
		let mut gossip_messages = Box::pin(
			self.gossip_engine
				.messages_for(topic::<B>())
				.filter_map(|notification| async move {
					trace!(target: "orderbook", "🥩 Got gossip message: {:?}", notification);

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
						if let Err(err) = self.handle_gossip_message(message) {
							debug!(target: "orderbook", "🥩 {}", err);
						}
					} else {
						return;
					}
				},
				_ = gossip_engine => {
					error!(target: "orderbook", "🥩 Gossip engine has terminated.");
					return;
				}
			}
		}
	}
}
