use super::*;
use crate::{
	gossip::{topic, GossipValidator},
	tests::{make_gradpa_ids, withdrawal::DummyForeignConnector},
};
use sc_network_gossip::GossipEngine;
use std::collections::HashMap;
use substrate_test_runtime_client::Ed25519Keyring;

#[tokio::test]
async fn dropped_one_validator_still_works_test() {
	sp_tracing::try_init_simple();

	let network = 1;
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_thea_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let message = Message {
		block_no: 10,
		nonce: 1,
		data: vec![1, 2, 3],
		network: 1,
		is_key_change: false,
		validator_set_id: 0,
		validator_set_len: 3,
	};
	let grandpa_peers = &[Ed25519Keyring::Alice, Ed25519Keyring::Bob, Ed25519Keyring::Charlie];
	let genesys_authorities = make_gradpa_ids(grandpa_peers);

	let runtime = Arc::new(TestApi {
		genesys_authorities,
		authorities: BTreeMap::from([(
			network,
			ValidatorSet { set_id: 0, validators: active.clone() },
		)]),
		validator_set_id: 0,
		_next_authorities: BTreeMap::new(),
		network_pref: BTreeMap::from([
			(active[0].clone(), network),
			(active[1].clone(), network),
			(active[2].clone(), network),
		]),
		outgoing_messages: BTreeMap::from([((network, 1), message.clone())]),
		incoming_messages: Arc::new(RwLock::new(BTreeMap::new())),
		incoming_nonce: Arc::new(RwLock::new(BTreeMap::new())),
		_outgoing_nonce: BTreeMap::from([(network, 1)]),
	});

	let foreign_connector = Arc::new(DummyForeignConnector {
		authorities: HashMap::from([(0, active)]),
		incoming_nonce: Arc::new(RwLock::new(0)),
		incoming_messages: Arc::new(RwLock::new(HashMap::new())),
	});

	let mut testnet = TheaTestnet::new(3, 0, runtime.clone());

	let validators = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth, foreign_connector.clone()))
		.collect();

	let grandpa_handle = tokio::spawn(initialize_grandpa(&mut testnet, grandpa_peers));
	let networking = testnet.peer(0).network_service().clone();
	let thea_handle = tokio::spawn(initialize_thea(&mut testnet, validators).await);

	// add new block
	testnet.peer(0).push_blocks(1, false);
	testnet.run_until_sync().await;
	// kill off one worker
	testnet.drop_validator();

	// push some message
	let message_cache = Arc::new(RwLock::new(BTreeMap::new()));
	let foreign_nonce = Arc::new(RwLock::new(0));
	let native_nonce = Arc::new(RwLock::new(0));
	let gossip_validator = Arc::new(GossipValidator::<Block>::new(
		message_cache.clone(),
		foreign_nonce.clone(),
		native_nonce.clone(),
	));
	let mut gossip_engine = GossipEngine::<Block>::new(
		networking.clone(),
		sc_network::ProtocolName::from(crate::thea_protocol_name::NAME),
		gossip_validator,
		None,
	);
	let message = Message {
		block_no: 1,
		nonce: 1,
		data: vec![1, 2, 3],
		network: 1,
		is_key_change: false,
		validator_set_id: 0,
		validator_set_len: 3u64,
	};
	gossip_engine.gossip_message(topic::<Block>(), message.encode(), false);

	testnet.run_until_sync().await;
	testnet.run_until_idle().await;

	// validate finality
	for i in 0..2 {
		assert_eq!(testnet.peer(i).client().info().best_number, 1, "Peer #{} failed to sync", i);
	}

	// verify process message
	assert!(!testnet.worker_massages.is_empty());
	assert_eq!(testnet.worker_massages.len(), 3);
	let mut retry = 0;
	loop {
		if retry >= 12 || !runtime.incoming_messages.read().is_empty() {
			break
		}
		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
		retry += 1;
	}
	assert!(retry < 12, "No incomming messages registered");
	// signing done and submitted
	assert!(
		!foreign_connector.incoming_messages.read().is_empty(),
		"No signature submitted to foreign chain"
	);

	// terminate
	thea_handle.abort_handle().abort();
	grandpa_handle.abort_handle().abort();
}

#[tokio::test]
async fn validator_set_change_mid_messaging_works_test() {
	sp_tracing::try_init_simple();

	let network = 1;
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_thea_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let message = Message {
		block_no: 10,
		nonce: 1,
		data: vec![1, 2, 3],
		network,
		is_key_change: false,
		validator_set_id: 0,
		validator_set_len: 3,
	};
	let grandpa_peers = &[Ed25519Keyring::Alice, Ed25519Keyring::Bob, Ed25519Keyring::Charlie];
	let genesys_authorities = make_gradpa_ids(grandpa_peers);

	let runtime = Arc::new(TestApi {
		genesys_authorities,
		authorities: BTreeMap::from([(
			network,
			ValidatorSet { set_id: 0, validators: active.clone() },
		)]),
		validator_set_id: 0,
		_next_authorities: BTreeMap::new(),
		network_pref: BTreeMap::from([
			(active[0].clone(), network),
			(active[1].clone(), network),
			(active[2].clone(), network),
		]),
		outgoing_messages: BTreeMap::from([((network, 1), message.clone())]),
		incoming_messages: Arc::new(RwLock::new(BTreeMap::new())),
		incoming_nonce: Arc::new(RwLock::new(BTreeMap::new())),
		_outgoing_nonce: BTreeMap::from([(network, 1)]),
	});

	let foreign_connector = Arc::new(DummyForeignConnector {
		authorities: HashMap::from([(0, active)]),
		incoming_nonce: Arc::new(RwLock::new(0)),
		incoming_messages: Arc::new(RwLock::new(HashMap::new())),
	});

	let mut testnet = TheaTestnet::new(3, 0, runtime.clone());

	let validators = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth, foreign_connector.clone()))
		.collect();

	let grandpa_handle = tokio::spawn(initialize_grandpa(&mut testnet, grandpa_peers));
	let networking = testnet.peer(0).network_service().clone();
	let thea_handle = tokio::spawn(initialize_thea(&mut testnet, validators).await);

	// add new block
	testnet.peer(0).push_blocks(1, false);
	testnet.run_until_sync().await;

	// rotate id

	// send foreign message with old id

	// verify local parsing still works

	// verify key and id updated on foreign chain

	// terminate
	thea_handle.abort_handle().abort();
	grandpa_handle.abort_handle().abort();
}
