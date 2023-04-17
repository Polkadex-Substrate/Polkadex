use crate::tests::{generate_and_finalize_blocks, initialize_orderbook, make_ob_ids, ObTestnet, TestApi};
use orderbook_primitives::crypto::AuthorityId;
use sp_consensus::BlockOrigin;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use std::sync::Arc;
use std::time::Duration;
use sc_network_test::TestNetFactory;

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
	// Generate and finalize one block
	generate_and_finalize_blocks(1,&mut testnet).await;
	tokio::time::sleep(Duration::from_secs(1)).await;
	// Generate and finalize one block
	generate_and_finalize_blocks(1,&mut testnet).await;
	tokio::time::sleep(Duration::from_secs(1)).await;
}
