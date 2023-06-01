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

use crate::tests::{generate_and_finalize_blocks, make_ob_ids, ObTestnet, TestApi};
use futures::{future::BoxFuture, StreamExt};
use memory_db::MemoryDB;
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{AccountAsset, ObMessage, UserActions},
};
use orderbook_rpc::OrderbookRpc;
use parking_lot::RwLock;
use polkadex_primitives::{ingress::IngressMessages, AssetId};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use sc_client_api::BlockchainEvents;
use sc_keystore::LocalKeystore;
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use sp_keystore::CryptoStore;
use std::{collections::HashMap, sync::Arc};

#[tokio::test]
pub async fn test_orderbook_rpc() {
	sp_tracing::try_init_simple();

	let (orderbook_operator, _) = sp_core::ecdsa::Pair::generate();
	let mut testnet = ObTestnet::new(3, 0);
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_ob_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let main = AccountKeyring::Alice;
	let proxy1 = AccountKeyring::Bob;
	let proxy2 = AccountKeyring::Charlie;

	let runtime = Arc::new(TestApi {
		active,
		latest_snapshot_nonce: Arc::new(Default::default()),
		snapshots: Arc::new(Default::default()),
		unprocessed: Arc::new(Default::default()),
		main_to_proxy_mapping: HashMap::from([(
			main.to_account_id(),
			vec![proxy1.to_account_id(), proxy2.to_account_id()],
		)]),
		pending_snapshot: None,
		operator_key: Some(orderbook_operator.public()),
		trading_config: vec![],
		withdrawals: Arc::new(Default::default()),
		ingress_messages: vec![
			IngressMessages::RegisterUser(main.to_account_id(), proxy1.to_account_id()),
			IngressMessages::Deposit(
				main.to_account_id(),
				AssetId::Polkadex,
				Decimal::from_f64(1.456).unwrap(),
			),
			IngressMessages::AddProxy(main.to_account_id(), proxy2.to_account_id()),
		],
		allowlisted_assets: vec![AssetId::Polkadex],
	});

	let peer_id = 0;
	let (sender, receiver) = futures::channel::mpsc::unbounded();
	testnet.peers[peer_id].data.peer_rpc_link = Some(sender.clone());
	testnet.peers[peer_id].data.is_validator = true;
	testnet.peers[peer_id].data.last_successful_block_number_snapshot_created =
		Arc::new(RwLock::new(0_u32.saturated_into()));
	testnet.peers[peer_id].data.memory_db = Arc::new(RwLock::new(MemoryDB::default()));
	testnet.peers[peer_id].data.working_state_root = Arc::new(RwLock::new([0; 32]));

	let key = AccountKeyring::Dave;

	// Generate the crypto material with test keys,
	// we have to use file based keystore,
	// in memory keystore doesn't seem to work here
	let keystore =
		Some(Arc::new(LocalKeystore::open(format!("keystore-{:?}", peer_id), None).unwrap()));
	let (pair, _seed) =
		orderbook_primitives::crypto::Pair::from_string_with_seed(&key.to_seed(), None).unwrap();
	// Insert the key
	keystore
		.as_ref()
		.unwrap()
		.insert_unknown(orderbook_primitives::KEY_TYPE, &key.to_seed(), pair.public().as_ref())
		.await
		.unwrap();
	// Check if the key is present or not
	keystore
		.as_ref()
		.unwrap()
		.key_pair::<orderbook_primitives::crypto::Pair>(&pair.public())
		.unwrap();

	let sync_oracle = testnet.peers[peer_id].network_service().clone();

	let deps = orderbook_rpc::OrderbookDeps {
		rpc_channel: sender,
		memory_db: testnet.peers[peer_id].data.memory_db.clone(),
		working_state_root: testnet.peers[peer_id].data.working_state_root.clone(),
		backend: testnet.peers[peer_id].client().as_backend().clone(),
		client: testnet.peers[peer_id].client().as_client().clone(),
		runtime: runtime.clone(),
	};
	let rpc_handle = OrderbookRpc::new(Arc::new(DummyTaskExecutor), deps);
	let worker_params = crate::worker::WorkerParams {
		client: testnet.peers[peer_id].client().as_client(),
		backend: testnet.peers[peer_id].client().as_backend(),
		runtime,
		sync_oracle,
		is_validator: true,
		network: testnet.peers[peer_id].network_service().clone(),
		protocol_name: "/ob/1".into(),
		message_sender_link: receiver,
		metrics: None,
		_marker: Default::default(),
		memory_db: testnet.peers[peer_id].data.memory_db.clone(),
		working_state_root: testnet.peers[peer_id].data.working_state_root.clone(),
		keystore,
	};

	let mut finality_stream_future = testnet.peers[peer_id]
		.client()
		.as_client()
		.finality_notification_stream()
		.fuse();

	let mut worker = crate::worker::ObWorker::<_, _, _, _, _, _>::new(worker_params);

	// Send the RPC with Ob message
	let mut message = ObMessage {
		worker_nonce: 1,
		stid: 10,
		action: UserActions::BlockImport(1),
		signature: Default::default(),
	};
	message.signature = orderbook_operator.sign_prehashed(&message.sign_data());
	// Generate one block
	generate_and_finalize_blocks(1, &mut testnet, 1).await;
	let next_finalized_blk = finality_stream_future.next().await.unwrap();

	// Progress the worker's chain
	worker.handle_finality_notification(&next_finalized_blk).await.unwrap();

	worker.process_new_user_action(&message).await.unwrap();

	let result: String = rpc_handle.get_orderbook_recovery_state_inner().await.unwrap();

	let offchain_state: orderbook_primitives::recovery::ObRecoveryState =
		serde_json::from_str(&result).unwrap();
	// Assert everything.

	assert_eq!(offchain_state.worker_nonce, 0); // We didn't generate any snapshot yet.
	assert_eq!(offchain_state.state_change_id, 0); // We didn't generate any snapshot yet.
	assert_eq!(offchain_state.snapshot_id, 0); // We didn't generate any snapshot yet.
	assert_eq!(offchain_state.account_ids.len(), 1);
	assert_eq!(
		offchain_state.account_ids.get(&main.to_account_id()).unwrap(),
		&vec![proxy1.to_account_id(), proxy2.to_account_id()]
	);
	assert_eq!(
		offchain_state
			.balances
			.get(&AccountAsset { main: main.to_account_id(), asset: AssetId::Polkadex })
			.cloned()
			.unwrap(),
		Decimal::from_f64(1.456).unwrap()
	);
}

#[derive(Clone)]
pub struct DummyTaskExecutor;

impl sp_core::traits::SpawnNamed for DummyTaskExecutor {
	fn spawn_blocking(
		&self,
		_name: &'static str,
		_group: Option<&'static str>,
		_future: BoxFuture<'static, ()>,
	) {
		todo!()
	}

	fn spawn(
		&self,
		_name: &'static str,
		_group: Option<&'static str>,
		_future: BoxFuture<'static, ()>,
	) {
		todo!()
	}
}
