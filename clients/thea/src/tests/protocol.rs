use super::*;
use crate::tests::{make_gradpa_ids, three_grandpa_validators, withdrawal::DummyForeignConnector};
use std::collections::HashMap;
use substrate_test_runtime_client::Ed25519Keyring;

#[tokio::test]
async fn dropped_one_validator_still_works() {
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
		network: 0,
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

	let mut testnet = TheaTestnet::new(3, 1, runtime.clone());
	let (grandpa_handle, mut grandpa_net) =
		three_grandpa_validators(runtime.clone(), grandpa_peers.as_ref());

	let validators = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth, foreign_connector.clone()))
		.collect();

	let thea_handle = tokio::spawn(initialize_thea(&mut testnet, validators).await);

	// kill off one worker
	testnet.drop_validator();

	// add new block
	grandpa_net.peer(0).push_blocks(1, false);
	grandpa_net.run_until_sync().await;

	// validate finality on thea side
	for i in 0..3 {
		assert_eq!(
			grandpa_net.peer(i).client().info().best_number,
			1,
			"Peer #{} failed to sync",
			i
		);
	}

	// verify process message
	for validator_index in 0..testnet.peers.len() {
		assert_eq!(testnet.worker_massages.get(&validator_index).unwrap().read().len(), 1);
	}

	// terminate
	thea_handle.abort_handle().abort();
	grandpa_handle.abort_handle().abort();
}
