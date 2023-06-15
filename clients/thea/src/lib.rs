// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! This crate responsible for observing foreign networks and aggregation of the BLS signatures for
//! ingress messages.
//! Creates external network payloads based on the inputs of solochain runtime.
//!
//! The client listens to 3 streams for messages:
//! * finality stream
//! * gossip messages
//! * interval stream

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
	use sc_chain_spec::ChainSpec;

	/// Protocol name.
	pub(crate) const NAME: &str = "/thea/1";

	/// Name of the notifications protocol used by BEEFY.
	///
	/// Must be registered towards the networking in order for BEEFY to properly function.
	pub fn standard_name<Hash: AsRef<[u8]>>(
		genesis_hash: &Hash,
		chain_spec: &dyn ChainSpec,
	) -> sc_network::ProtocolName {
		let chain_prefix = match chain_spec.fork_id() {
			Some(fork_id) => format!("/{}/{}", hex::encode(genesis_hash), fork_id),
			None => format!("/{}", hex::encode(genesis_hash)),
		};
		format!("{chain_prefix}{NAME}").into()
	}
}

/// Returns the configuration value to put in
/// [`sc_network::config::NetworkConfiguration::extra_sets`].
/// For standard protocol name see [`thea_protocol_name::standard_name`].
pub fn thea_peers_set_config(
	protocol_name: sc_network::ProtocolName,
) -> sc_network::config::NonDefaultSetConfig {
	let mut cfg = sc_network::config::NonDefaultSetConfig::new(protocol_name, 1024 * 1024);

	cfg.allow_non_reserved(25, 25);
	cfg
}

/// A convenience Thea client trait that defines all the type bounds a Thea client
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
use sc_network_gossip::{Network as GossipNetwork, Syncing};

/// Thea gadget initialization parameters.
pub struct TheaParams<B, BE, C, N, R, SO>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	SO: SyncOracle + Syncing<B>,
{
	/// Thea client.
	pub client: Arc<C>,
	/// Client Backend.
	pub backend: Arc<BE>,
	/// Client runtime.
	pub runtime: Arc<R>,
	/// Keystore.
	pub keystore: Arc<LocalKeystore>,
	/// Gossip network.
	pub network: N,
	/// Sync service
	pub sync_oracle: Arc<SO>,
	/// Prometheus metric registry.
	pub prometheus_registry: Option<Registry>,
	/// Boolean indicating if this node is a validator.
	pub is_validator: bool,
	pub marker: PhantomData<B>,
	/// Defines the chain type our current deployment (Dev or production).
	pub chain_type: ChainType,
	/// Foreign Chain URL.
	pub foreign_chain_url: String,
	/// Foreign chain dummy mode
	pub dummy_mode: bool,
}

/// Start the Thea gadget.
///
/// This is a thin shim around running and awaiting a Thea worker.
pub async fn start_thea_gadget<B, BE, C, N, R, SO>(ob_params: TheaParams<B, BE, C, N, R, SO>)
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
	N: GossipNetwork<B> + Clone + Send + Sync + 'static,
	SO: Clone + Send + Sync + 'static + SyncOracle + Syncing<B>,
{
	let TheaParams {
		client,
		backend,
		runtime,
		keystore,
		network,
		sync_oracle,
		prometheus_registry,
		is_validator,
		marker: _,
		chain_type,
		foreign_chain_url,
		dummy_mode,
	} = ob_params;

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

	let foreign_connector = get_connector(chain_type, is_validator, foreign_chain_url, dummy_mode)
		.await
		.connector;

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

/// Foreign connector wrapper. Holds concrete implementation of the foreign connector abstraction.
pub struct Connector {
	connector: Arc<dyn ForeignConnector>,
}

/// Connector resolver/factory.
///
/// Based on chain type or validators group member - resolves, creates and returns a new
/// connector instance.
///
/// # Parameters
///
/// * `chain_type`: Type of chain for which connector should be created.
/// * `is_validator`: Defines if connector should be created for validator or not.
/// * `url`: The address to which connector should be connected.
pub async fn get_connector(
	chain_type: ChainType,
	is_validator: bool,
	url: String,
	dummy_mode: bool,
) -> Connector {
	log::info!(target:"thea","🌉 Assigning connector based on chain type: {:?}",chain_type);
	if !is_validator | dummy_mode {
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
