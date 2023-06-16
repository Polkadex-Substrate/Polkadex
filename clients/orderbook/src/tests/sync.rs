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

use crate::tests::{
	generate_and_finalize_blocks, initialize_orderbook, make_ob_ids, ObTestnet, TestApi,
};
use futures::SinkExt;
use memory_db::MemoryDB;
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{ObMessage, UserActions},
};
use parking_lot::RwLock;
use polkadex_primitives::{ingress::IngressMessages, AccountId, AssetId};
use primitive_types::H256;
use rust_decimal::Decimal;
use sc_network::NetworkStateInfo;
use sc_network_test::{FullPeerConfig, TestNetFactory};
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use sp_runtime::traits::IdentifyAccount;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::info_span;
use tracing_futures::Instrument;

#[ignore]
#[tokio::test]
pub async fn test_orderbook_snapshot() {
	// sp_tracing::try_init_simple();
	tracing_subscriber::fmt::init();

	let (orderbook_operator, _) = sp_core::ecdsa::Pair::generate();
	let main: AccountId = AccountKeyring::Alice.public().into_account().into();
	let proxy: AccountId = AccountKeyring::Bob.public().into_account().into();
	let mut testnet = ObTestnet::new(3, 2);
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
		(AccountKeyring::Dave, false),
	];

	let active: Vec<AuthorityId> =
		make_ob_ids(&peers.iter().map(|(k, _)| *k).collect::<Vec<AccountKeyring>>());

	let runtime = Arc::new(TestApi {
		active,
		latest_snapshot_nonce: Arc::new(Default::default()),
		snapshots: Arc::new(Default::default()),
		unprocessed: Arc::new(Default::default()),
		main_to_proxy_mapping: HashMap::default(),
		pending_snapshot: None,
		operator_key: Some(orderbook_operator.public()),
		trading_config: vec![],
		withdrawals: Arc::new(Default::default()),
		ingress_messages: vec![
			IngressMessages::RegisterUser(main.clone(), proxy.clone()),
			IngressMessages::Deposit(main.clone(), AssetId::Polkadex, Decimal::new(10, 0)),
			IngressMessages::Deposit(main.clone(), AssetId::Asset(1), Decimal::new(10, 0)),
		],
		allowlisted_assets: vec![AssetId::Polkadex],
	});

	let ob_peers = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth))
		.collect();

	let future = initialize_orderbook(&mut testnet, ob_peers).await;
	tokio::spawn(future);
	// Generate and finalize two block to start finality
	generate_and_finalize_blocks(1, &mut testnet, 3).await;
	generate_and_finalize_blocks(1, &mut testnet, 3).await;
	// Send the RPC with Ob message
	let mut message = ObMessage {
		worker_nonce: 1,
		stid: 10,
		action: UserActions::BlockImport(1),
		signature: Default::default(),
		reset: false,
		version: 0,
	};
	message.signature = orderbook_operator.sign_prehashed(&message.sign_data());
	testnet.peers[0]
		.data
		.peer_rpc_link
		.as_ref()
		.unwrap()
		.send(message)
		.await
		.unwrap();
	testnet.run_until_sync().await;

	// Generate and finalize one block
	generate_and_finalize_blocks(5, &mut testnet, 3).await;

	testnet.run_until_idle().await;

	// We should have generated one snapshot by this point
	assert_eq!(runtime.snapshots.read().len(), 1);
	for peer in testnet.peers() {
		let state_root = H256::from_slice(&*peer.data.working_state_root.read());
		if peer.data.is_validator {
			println!(
				"Validator id: {:?}, root: {:?}",
				peer.network_service().local_peer_id(),
				state_root
			);
			assert_eq!(state_root, runtime.get_latest_snapshot().state_root);
		} else {
			println!(
				"Fullnode id: {:?}, root: {:?}",
				peer.network_service().local_peer_id(),
				state_root
			);
		}
	}

	// Add the new full node, this is the fifth node
	testnet.add_full_peer_with_config(FullPeerConfig {
		notifications_protocols: vec!["/ob/1".into()],
		is_authority: false,
		..Default::default()
	});
	// Start the new node's worker
	let fifth_node_index = testnet.peers().len() - 1;

	let working_state_root = Arc::new(RwLock::new([0; 32]));

	let memory_db = Arc::new(RwLock::new(MemoryDB::default()));
	let (_sender, receiver) = futures::channel::mpsc::unbounded();
	// Now we add a new full node and see if it can catch up.
	let ob_params = crate::ObParams {
		client: testnet.peers[fifth_node_index].client().as_client(),
		backend: testnet.peers[fifth_node_index].client().as_backend(),
		runtime: runtime.clone(),
		keystore: Arc::new(sc_keystore::LocalKeystore::in_memory()),
		network: testnet.peers[fifth_node_index].network_service().clone(),
		sync: testnet.peers[fifth_node_index].sync_service().clone(),
		prometheus_registry: None,
		protocol_name: "/ob/1".into(),
		is_validator: false,
		message_sender_link: receiver,
		marker: Default::default(),
		memory_db: memory_db.clone(),
		working_state_root: working_state_root.clone(),
	};

	let gadget = crate::start_orderbook_gadget::<_, _, _, _, _, _>(ob_params)
		.instrument(info_span!("ful:", fifth_node_index));

	testnet.run_until_connected().await;
	// Start the worker.
	tokio::spawn(gadget);
	// Generate and finalize one block
	generate_and_finalize_blocks(3, &mut testnet, 3).await;
	// Let the testnet sync
	testnet.run_until_sync().await;
	// Let the network activity settle down.
	testnet.run_until_idle().await;

	tokio::time::sleep(Duration::new(10, 0)).await;

	// TODO: Fix this in the next release. The fullnode is not getting Chunk(data) gossip message
	// 		because of a size constraint. If we replace the data with a smaller vector it is working in
	// test The fullnodes are not receiving gossip in unit tests.
	let working_root = working_state_root.read();
	// Assert if the fullnode's working state root is updated.
	assert_eq!(sp_core::H256::from_slice(&*working_root), runtime.get_latest_snapshot().state_root)
}
