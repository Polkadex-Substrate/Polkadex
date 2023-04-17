use crate::tests::{
	generate_and_finalize_blocks, initialize_orderbook, make_ob_ids, ObTestnet, TestApi,
};
use futures::SinkExt;
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{ObMessage, UserActions},
};
use sc_network_test::TestNetFactory;
use sp_consensus::BlockOrigin;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use std::{sync::Arc, time::Duration};

#[tokio::test]
pub async fn test_orderbook_sync() {
	env_logger::init();

	let (orderbook_operator, _) = sp_core::ecdsa::Pair::generate();
	let mut testnet = ObTestnet::new(3, 0);
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_ob_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	println!("Acive list: {:?}", active);

	let validator_api = Arc::new(TestApi {
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
		.map(|(id, (key, is_auth))| (id, key, validator_api.clone(), *is_auth))
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
	assert_eq!(validator_api.snapshots.read().len(), 1);
}
