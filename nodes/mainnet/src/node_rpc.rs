// This file is part of Substrate.
//
// Copyright (C) 2019-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A collection of runtime-node-specific RPC methods.
//!
//! Since `substrate` core functionality makes no assumptions
//! about the modules used inside the runtime, so do
//! RPC methods defined in `sc-rpc` crate.
//! It means that `client/rpc` can't have any methods that
//! need some strong assumptions about the particular runtime.
//!
//! The RPCs available in this crate however can make some assumptions
//! about how the runtime is constructed and what FRAME pallets
//! are part of it. Therefore all runtime-specific
//! RPCs can be placed here or imported from corresponding FRAME RPC definitions.

#![warn(missing_docs)]

use jsonrpsee::RpcModule;
use pallet_ocex_rpc::PolkadexOcexRpc;
use pallet_rewards_rpc::PolkadexRewardsRpc;
use pallet_ocex_rpc::PolkadexOcexRuntimeApi;
use thea_executor_rpc::PolkadexSwapRpc;
use pallet_asset_conversion::AssetConversionApi;
use thea_executor_rpc::PolkadexSwapRpcApiServer;

use grandpa::{
	FinalityProofProvider, GrandpaJustificationStream, SharedAuthoritySet, SharedVoterState,
};
use polkadex_primitives::{AccountId, Balance, Block, BlockNumber, Hash, Index};
use rpc_assets::{PolkadexAssetHandlerRpc, PolkadexAssetHandlerRpcApiServer};
use sc_client_api::{AuxStore, BlockchainEvents};
use sc_consensus_babe::BabeWorkerHandle;
use sc_rpc::{statement::StatementApiServer, SubscriptionTaskExecutor};
/// Re-export the API for backward compatibility.
pub use sc_rpc_api::offchain::*;
pub use sc_rpc_api::DenyUnsafe;
use sc_transaction_pool_api::TransactionPool;
use sp_api::ProvideRuntimeApi;
use sp_block_builder::BlockBuilder;
use sp_blockchain::{Error as BlockChainError, HeaderBackend, HeaderMetadata};
use sp_consensus::SelectChain;
use sp_consensus_babe::BabeApi;
use sp_keystore::KeystorePtr;
use std::sync::Arc;

/// Extra dependencies for BABE.
pub struct BabeDeps {
	/// A handle to the BABE worker for issuing requests.
	pub babe_worker_handle: BabeWorkerHandle<Block>,
	/// The keystore that manages the keys of the runtime-node.
	pub keystore: KeystorePtr,
}

/// Extra dependencies for GRANDPA
pub struct GrandpaDeps<B> {
	/// Voting round info.
	pub shared_voter_state: SharedVoterState,
	/// Authority set info.
	pub shared_authority_set: SharedAuthoritySet<Hash, BlockNumber>,
	/// Receives notifications about justification events from Grandpa.
	pub justification_stream: GrandpaJustificationStream<Block>,
	/// Executor to drive the subscription manager in the Grandpa RPC handler.
	pub subscription_executor: SubscriptionTaskExecutor,
	/// Finality proof provider.
	pub finality_provider: Arc<FinalityProofProvider<B, Block>>,
}

/// Full client dependencies.
pub struct FullDeps<C, P, SC, B> {
	/// The client instance to use.
	pub client: Arc<C>,
	/// Transaction pool instance.
	pub pool: Arc<P>,
	/// The SelectChain Strategy
	pub select_chain: SC,
	/// A copy of the chain spec.
	pub chain_spec: Box<dyn sc_chain_spec::ChainSpec>,
	/// Whether to deny unsafe calls
	pub deny_unsafe: DenyUnsafe,
	/// BABE specific dependencies.
	pub babe: BabeDeps,
	/// GRANDPA specific dependencies.
	pub grandpa: GrandpaDeps<B>,
	/// Shared statement store reference.
	pub statement_store: Arc<dyn sp_statement_store::StatementStore>,
	/// The backend used by the runtime-node.
	pub backend: Arc<B>,
}

/// Instantiate all Full RPC extensions.
pub fn create_full<C, P, SC, B>(
	deps: FullDeps<C, P, SC, B>,
) -> Result<RpcModule<()>, Box<dyn std::error::Error + Send + Sync>>
where
	C: ProvideRuntimeApi<Block>
		+ sc_client_api::BlockBackend<Block>
		+ HeaderBackend<Block>
		+ AuxStore
		+ HeaderMetadata<Block, Error = BlockChainError>
		+ Sync
		+ Send
		+ 'static,
	C::Api: substrate_frame_rpc_system::AccountNonceApi<Block, AccountId, Index>,
	C::Api: pallet_transaction_payment_rpc::TransactionPaymentRuntimeApi<Block, Balance>,
	C::Api: BabeApi<Block>,
	C::Api: BlockBuilder<Block>,
	P: TransactionPool + 'static,
	SC: SelectChain<Block> + 'static,
	B: sc_client_api::Backend<Block> + Send + Sync + 'static,
	B::State: sc_client_api::backend::StateBackend<sp_runtime::traits::HashingFor<Block>>,
	C::Api: rpc_assets::PolkadexAssetHandlerRuntimeApi<Block, AccountId, Hash>,
	C::Api: pallet_rewards_rpc::PolkadexRewardsRuntimeApi<Block, AccountId, Hash>,
	C::Api: pallet_asset_conversion::AssetConversionApi<Block, Balance, u128, pallet_asset_conversion::NativeOrAssetId<u128>>,
	C::Api: PolkadexOcexRuntimeApi<Block, AccountId, Hash>,
	C: BlockchainEvents<Block>,
{
	use pallet_ocex_rpc::PolkadexOcexRpcApiServer;
	use pallet_rewards_rpc::PolkadexRewardsRpcApiServer;
	use pallet_transaction_payment_rpc::{TransactionPayment, TransactionPaymentApiServer};
	use sc_consensus_babe_rpc::{Babe, BabeApiServer};
	use sc_consensus_grandpa_rpc::{Grandpa, GrandpaApiServer};
	use sc_rpc::dev::{Dev, DevApiServer};
	use sc_sync_state_rpc::{SyncState, SyncStateApiServer};
	use substrate_frame_rpc_system::{System, SystemApiServer};
	// use substrate_state_trie_migration_rpc::{StateMigration, StateMigrationApiServer};

	let mut io = RpcModule::new(());
	let FullDeps {
		client,
		pool,
		select_chain,
		chain_spec,
		deny_unsafe,
		babe,
		grandpa,
		statement_store,
		backend,
	} = deps;

	let BabeDeps { keystore, babe_worker_handle } = babe;
	let GrandpaDeps {
		shared_voter_state,
		shared_authority_set,
		justification_stream,
		subscription_executor,
		finality_provider,
	} = grandpa;

	io.merge(System::new(client.clone(), pool, deny_unsafe).into_rpc())?;
	io.merge(TransactionPayment::new(client.clone()).into_rpc())?;
	io.merge(
		Babe::new(client.clone(), babe_worker_handle.clone(), keystore, select_chain, deny_unsafe)
			.into_rpc(),
	)?;
	io.merge(
		Grandpa::new(
			subscription_executor,
			shared_authority_set.clone(),
			shared_voter_state,
			justification_stream,
			finality_provider,
		)
		.into_rpc(),
	)?;

	io.merge(
		SyncState::new(chain_spec, client.clone(), shared_authority_set, babe_worker_handle)?
			.into_rpc(),
	)?;

	// io.merge(StateMigration::new(client.clone(), backend, deny_unsafe).into_rpc())?;
	io.merge(PolkadexAssetHandlerRpc::new(client.clone()).into_rpc())?;
	io.merge(PolkadexRewardsRpc::new(client.clone()).into_rpc())?;
	io.merge(PolkadexSwapRpc::new(
		client.clone(),
		deny_unsafe,
	)
		.into_rpc())?;
	io.merge(
		PolkadexOcexRpc::new(
			client.clone(),
			backend
				.offchain_storage()
				.ok_or("Backend doesn't provide an offchain storage")?,
			deny_unsafe,
		)
		.into_rpc())?;

	// io.merge(PolkadexSwapRpc::new(client.clone(),
	// 							  backend
	// 								  .offchain_storage()
	// 								  .ok_or("Backend doesn't provide an offchain storage")?,
	// 							  deny_unsafe,
	// )
	// 			 .into_rpc(),
	// )?;
	io.merge(Dev::new(client.clone(), deny_unsafe).into_rpc())?;
	let statement_store =
		sc_rpc::statement::StatementStore::new(statement_store, deny_unsafe).into_rpc();
	io.merge(statement_store)?;
	Ok(io)
}
