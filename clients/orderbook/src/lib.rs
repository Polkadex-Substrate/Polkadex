#![feature(unwrap_infallible)]
extern crate core;

use futures::channel::mpsc::UnboundedReceiver;
use orderbook_primitives::{ObApi, SnapshotSummary};
pub use orderbook_protocol_name::standard_name as protocol_standard_name;

use memory_db::{HashKey, MemoryDB};
use parking_lot::RwLock;
use prometheus::Registry;
use reference_trie::{ExtensionLayout, RefHasher};
use sc_client_api::{Backend, BlockchainEvents, FinalityNotification, Finalizer};
use sc_network::PeerId;
use sc_network_common::protocol::event::Event;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::SyncOracle;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::Block;
use std::{future::Future, marker::PhantomData, sync::Arc};

mod error;
mod gossip;
mod metrics;
mod utils;
mod worker;

// PR 675 will fix test cases
// #[cfg(test)]
// mod tests;

pub(crate) mod orderbook_protocol_name {
	use sc_chain_spec::ChainSpec;

	const NAME: &str = "/ob/1";

	/// Name of the notifications protocol used by BEEFY.
	///
	/// Must be registered towards the networking in order for BEEFY to properly function.
	pub fn standard_name<Hash: AsRef<[u8]>>(
		genesis_hash: &Hash,
		chain_spec: &Box<dyn ChainSpec>,
	) -> std::borrow::Cow<'static, str> {
		let chain_prefix = match chain_spec.fork_id() {
			Some(fork_id) => format!("/{}/{}", hex::encode(genesis_hash), fork_id),
			None => format!("/{}", hex::encode(genesis_hash)),
		};
		format!("{}{}", chain_prefix, NAME).into()
	}
}

/// Returns the configuration value to put in
/// [`sc_network::config::NetworkConfiguration::extra_sets`].
/// For standard protocol name see [`orderbook_protocol_name::standard_name`].
pub fn orderbook_peers_set_config(
	protocol_name: std::borrow::Cow<'static, str>,
) -> sc_network::config::NonDefaultSetConfig {
	let mut cfg = sc_network::config::NonDefaultSetConfig::new(protocol_name, 1024 * 1024);

	cfg.allow_non_reserved(25, 25);
	cfg
}

/// A convenience Orderbook client trait that defines all the type bounds a Orderbook client
/// has to satisfy. Ideally that should actually be a trait alias. Unfortunately as
/// of today, Rust does not allow a type alias to be used as a trait bound. Tracking
/// issue is <https://github.com/rust-lang/rust/issues/41517>.
pub trait Client<B, BE>:
	BlockchainEvents<B> + HeaderBackend<B> + Finalizer<B, BE> + Send + Sync
where
	B: Block,
	BE: Backend<B>,
{
	// empty
}

impl<B, BE, T> Client<B, BE> for T
where
	B: Block,
	BE: Backend<B>,
	T: BlockchainEvents<B>
		+ HeaderBackend<B>
		+ Finalizer<B, BE>
		+ ProvideRuntimeApi<B>
		+ Send
		+ Sync,
{
	// empty
}

use crate::error::Error;
use orderbook_primitives::types::{ObMessage, UserActions};
use polkadex_primitives::BlockNumber;
use sc_network_gossip::Network as GossipNetwork;

/// Orderbook gadget initialization parameters.
pub struct ObParams<B, BE, C, N, R>
where
	B: Block,
	BE: Backend<B>,
	R: ProvideRuntimeApi<B>,
	C: Client<B, BE>,
	R::Api: ObApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
{
	/// Orderbook client
	pub client: Arc<C>,
	/// Client Backend
	pub backend: Arc<BE>,
	/// Client runtime
	pub runtime: Arc<R>,
	/// Local key store
	pub key_store: Option<SyncCryptoStorePtr>,
	/// Gossip network
	pub network: N,
	/// Prometheus metric registry
	pub prometheus_registry: Option<Registry>,
	/// Chain specific Ob protocol name. See [`orderbook_protocol_name::standard_name`].
	pub protocol_name: std::borrow::Cow<'static, str>,
	/// Boolean indicating if this node is a validator
	pub is_validator: bool,
	/// Submit message link
	pub message_sender_link: UnboundedReceiver<ObMessage>,
	// Links between the block importer, the background voter and the RPC layer.
	// pub links: BeefyVoterLinks<B>,
	pub marker: PhantomData<B>,
	// lock 64
	pub last_successful_block_no_snapshot_created: Arc<RwLock<BlockNumber>>,
	// memory db
	pub memory_db: Arc<RwLock<MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>>>,
	// working state root
	pub working_state_root: Arc<RwLock<[u8; 32]>>,
}

/// Start the Orderbook gadget.
///
/// This is a thin shim around running and awaiting a Orderbook worker.
pub async fn start_orderbook_gadget<B, BE, C, N, R>(ob_params: ObParams<B, BE, C, N, R>)
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: ObApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static + SyncOracle,
{
	let ObParams {
		client,
		backend,
		runtime,
		key_store: _,
		network,
		prometheus_registry,
		protocol_name,
		is_validator,
		message_sender_link,
		marker: _,
		last_successful_block_no_snapshot_created,
		memory_db,
		working_state_root,
	} = ob_params;

	let sync_oracle = network.clone();

	let metrics =
		prometheus_registry.as_ref().map(metrics::Metrics::register).and_then(
			|result| match result {
				Ok(metrics) => {
					log::debug!(target: "orderbook", "🥩 Registered metrics");
					Some(metrics)
				},
				Err(err) => {
					log::debug!(target: "orderbook", "🥩 Failed to register metrics: {:?}", err);
					None
				},
			},
		);

	let worker_params = worker::WorkerParams {
		client,
		backend,
		runtime,
		sync_oracle,
		is_validator,
		network,
		protocol_name,
		message_sender_link,
		metrics,
		_marker: Default::default(),
	};

	// ToDo: Pass the parameters to the worker module
	let mut worker = worker::ObWorker::<_, _, _, _, _, _>::new(worker_params);

	worker.run().await
}
