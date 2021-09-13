// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
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

use std::sync::Arc;

use log::debug;

use sc_client_api::{Backend, BlockchainEvents, Finalizer};

use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::traits::Block;

use offchain_ipfs_primitives::IpfsApi;
use std::marker::PhantomData;
use codec::Codec;

mod worker;

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

pub struct IPFSParams<B, BE, C, AccountId>
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	C::Api: IpfsApi<B, AccountId>,
	AccountId: Codec,
{
	pub client: Arc<C>,
	pub backend: Arc<BE>,
	pub block: PhantomData<B>,
	pub accountid: PhantomData<AccountId>,
}

pub async fn start_offchain_ipfs<B, BE, C, AccountId>(ipfs_params: IPFSParams<B, BE, C,AccountId>)
where
	B: Block,
	BE: Backend<B>,
	C: Client<B, BE>,
	C::Api: IpfsApi<B,AccountId>,
	AccountId: Codec + Eq + std::hash::Hash,
{
	let IPFSParams { client,
		backend,
		block,
		accountid} = ipfs_params;

	let worker_params = worker::WorkerParams { client, backend, block };

	let worker = worker::IPFSWorker::<_, _, _,_>::new(worker_params).await;

	worker.run().await
}
