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
use sc_network_test::{FullPeerConfig, TestNetFactory};
use sp_arithmetic::traits::SaturatedConversion;
use sp_consensus::BlockOrigin;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use std::{sync::Arc, time::Duration};

#[tokio::test]
pub async fn test_orderbook_sync() {
	env_logger::init();

	let (orderbook_operator, _) = sp_core::ecdsa::Pair::generate();
	let mut testnet = ObTestnet::new(3, 1);
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_ob_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let runtime = Arc::new(TestApi {
		active,
		latest_snapshot_nonce: Arc::new(Default::default()),
		snapshots: Arc::new(Default::default()),
		unprocessed: Arc::new(Default::default()),
		main_to_proxy_mapping: Default::default(),
		pending_snapshot: None,
		operator_key: Some(orderbook_operator.public()),
		trading_config: vec![],
		withdrawals: Arc::new(Default::default()),
	});

	let ob_peers = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth))
		.collect();

	let future = initialize_orderbook(&mut testnet, ob_peers).await;
	tokio::spawn(future);
	// Generate and finalize two block to start finality
	generate_and_finalize_blocks(1, &mut testnet).await;
	generate_and_finalize_blocks(1, &mut testnet).await;
	// Send the RPC with Ob message
	let mut message =
		ObMessage { stid: 1, action: UserActions::BlockImport(1), signature: Default::default() };
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
	generate_and_finalize_blocks(5, &mut testnet).await;

	testnet.run_until_idle().await;
	// We should have generated one snapshot by this point
	assert_eq!(runtime.snapshots.read().len(), 1);
	// Add a new full node, this is the fifth node
	testnet.add_full_peer_with_config(FullPeerConfig {
		notifications_protocols: vec!["/ob/1".into()],
		is_authority: false,
		..Default::default()
	});

	let fifth_node_index = 4;

	let working_state_root = Arc::new(RwLock::new([0; 32]));

	let last_successful_block_number_snapshot_created =
		Arc::new(RwLock::new(0_u32.saturated_into()));

	let memory_db = Arc::new(RwLock::new(MemoryDB::default()));
	let (sender, receiver) = futures::channel::mpsc::unbounded();
	// Now we add a new full node and see if it can catch up.
	let ob_params = crate::ObParams {
		client: testnet.peers[fifth_node_index].client().as_client(),
		backend: testnet.peers[fifth_node_index].client().as_backend(),
		runtime,
		keystore: None,
		network: testnet.peers[fifth_node_index].network_service().clone(),
		prometheus_registry: None,
		protocol_name: "/ob/1".into(),
		is_validator: false,
		message_sender_link: receiver,
		marker: Default::default(),
		last_successful_block_number_snapshot_created:
			last_successful_block_number_snapshot_created.clone(),
		memory_db: memory_db.clone(),
		working_state_root: working_state_root.clone(),
	};
	let gadget = crate::start_orderbook_gadget::<_, _, _, _, _>(ob_params);
	// Start the worker.
	tokio::spawn(gadget);
	// Let the testnet sync
	testnet.run_until_sync().await;
	// Let the network activity settle down.
	testnet.run_until_idle().await;
}
