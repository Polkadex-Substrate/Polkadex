// Copyright (C) 2020-2021 Polkadex OU
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

// This is file is modified from beefy-gadget from Parity Technologies (UK) Ltd.

use log::*;
use sc_client_api::{Backend, BlockchainEvents, Finalizer};
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_keystore::SyncCryptoStorePtr;
use sp_runtime::traits::Block;
use std::sync::{Arc, Mutex};
use thea_primitives::TheaApi;

pub mod error;
pub mod inherents;
pub mod keystore;
mod rounds;
mod utils;
pub mod worker;
pub use rounds::RoundTracker;

#[cfg(test)]
mod tests;

/// A convenience THEA client trait that defines all the type bounds a THEA client
/// has to satisfy. Ideally that should actually be a trait alias. Unfortunately as
/// of today, Rust does not allow a type alias to be used as a trait bound. Tracking
/// issue is <https://github.com/rust-lang/rust/issues/41517>.
pub trait Client<B, BE>:
	BlockchainEvents<B> + HeaderBackend<B> + Finalizer<B, BE> + ProvideRuntimeApi<B> + Send + Sync
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

/// t-ECDSA Initialization Params
pub struct TheaParams<C, BE, R> {
	/// THEA client
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub runtime: Arc<R>,
	/// Local key store
	pub key_store: Option<SyncCryptoStorePtr>,
	pub rpc_send: Arc<Mutex<std::sync::mpsc::Sender<worker::RoundInfo>>>,
}

/// Start the THEA gadget.
///
/// This is a thin shim around running and awaiting a THEA worker.
pub async fn start_thea_gadget<B, BE, C, R>(thea_params: TheaParams<C, BE, R>)
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	R: ProvideRuntimeApi<B>,
	R::Api: TheaApi<B>,
{
	let TheaParams { client, backend, runtime, key_store, rpc_send } = thea_params;

	let worker_params = worker::WorkerParams { client, backend, runtime, key_store, rpc_send };

	let mut worker = worker::TheaWorker::<_, _, _, _>::new(worker_params);
	debug!(target: "thea", "Thea Worker Started!");
	worker.run().await
}
