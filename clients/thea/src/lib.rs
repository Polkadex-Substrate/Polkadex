#![feature(unwrap_infallible)]
use prometheus::Registry;
use sc_chain_spec::ChainType;
use sc_client_api::{Backend, BlockchainEvents, Finalizer};
use sc_keystore::LocalKeystore;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_consensus::SyncOracle;
use sp_runtime::traits::Block;
use std::{marker::PhantomData, sync::Arc};
use thea_primitives::TheaApi;
pub use thea_protocol_name::standard_name as protocol_standard_name;

mod error;
mod gossip;
mod metrics;
mod worker;

#[cfg(test)]
mod tests;

mod connector;
mod keystore;
mod types;

pub(crate) mod thea_protocol_name {

	pub(crate) const NAME: &str = "/thea/1";

	/// Name of the notifications protocol used by Thea.
	///
	/// Must be registered towards the networking in order for Thea to properly function.
	pub fn standard_name() -> sc_network::ProtocolName {
		sc_network::ProtocolName::Static(NAME)
	}
}

/// Returns the configuration value to put in
/// [`sc_network::config::NetworkConfiguration::extra_sets`].
/// For standard protocol name see [`thea_protocol_name::standard_name`].
pub fn thea_peers_set_config() -> sc_network_common::config::NonDefaultSetConfig {
	let mut cfg = sc_network_common::config::NonDefaultSetConfig::new(standard_name(), 1024 * 1024);

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

use crate::{
	connector::{
		parachain::ParachainClient,
		traits::{ForeignConnector, NoOpConnector},
	},
	thea_protocol_name::standard_name,
	worker::TheaWorker,
};
use sc_network_gossip::Network as GossipNetwork;

/// Thea gadget initialization parameters.
pub struct TheaParams<B, BE, C, N, R>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
{
	/// Orderbook client
	pub client: Arc<C>,
	/// Client Backend
	pub backend: Arc<BE>,
	/// Client runtime
	pub runtime: Arc<R>,
	/// Keystore
	pub keystore: Option<Arc<LocalKeystore>>,
	/// Gossip network
	pub network: N,
	/// Prometheus metric registry
	pub prometheus_registry: Option<Registry>,
	/// Boolean indicating if this node is a validator
	pub is_validator: bool,
	pub marker: PhantomData<B>,
	/// Defines the chain type our current deployment ( Dev or production )
	pub chain_type: ChainType,
	/// Foreign Chain URL
	pub foreign_chain_url: String,
}

/// Start the Thea gadget.
///
/// This is a thin shim around running and awaiting a Thea worker.
pub async fn start_thea_gadget<B, BE, C, N, R>(ob_params: TheaParams<B, BE, C, N, R>)
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static + SyncOracle,
{
	let TheaParams {
		client,
		backend,
		runtime,
		keystore,
		network,
		prometheus_registry,
		is_validator,
		marker: _,
		chain_type,
		foreign_chain_url,
	} = ob_params;

	let sync_oracle = network.clone();

	let metrics =
		prometheus_registry.as_ref().map(metrics::Metrics::register).and_then(
			|result| match result {
				Ok(metrics) => {
					log::debug!(target: "thea", "🌉 Registered metrics");
					Some(metrics)
				},
				Err(err) => {
					log::debug!(target: "thea", "🌉 Failed to register metrics: {:?}", err);
					None
				},
			},
		);

	let foreign_connector =
		get_connector(chain_type, is_validator, foreign_chain_url).await.connector;

	let worker_params = worker::WorkerParams {
		client,
		backend,
		runtime,
		keystore,
		sync_oracle,
		is_validator,
		network,
		metrics,
		_marker: Default::default(),
		foreign_chain: foreign_connector,
	};

	let worker = TheaWorker::<_, _, _, _, _, _, _>::new(worker_params).await;

	worker.run().await
}

pub struct Connector {
	connector: Arc<dyn ForeignConnector>,
}

pub async fn get_connector(chain_type: ChainType, is_validator: bool, url: String) -> Connector {
	log::info!(target:"thea","🌉 Assigning connector based on chain type: {:?}",chain_type);
	if !is_validator {
		return Connector { connector: Arc::new(NoOpConnector) }
	}
	match chain_type {
		ChainType::Development => Connector { connector: Arc::new(NoOpConnector) },
		_ => Connector {
			connector: Arc::new(
				ParachainClient::connect(url)
					.await
					.expect("🌉 Expected to connect to local foreign node"),
			),
		},
	}
}
