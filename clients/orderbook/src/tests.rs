//! This module contains code that defines test cases related to a offline storage of worker module.
use primitive_types::H128;
use std::{borrow::Cow, future::Future, sync::Arc};

use futures::{channel::mpsc::UnboundedSender, stream::FuturesUnordered, StreamExt};
use memory_db::{HashKey, MemoryDB};
use parity_scale_codec::{Decode, Encode};
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::{prelude::FromPrimitive, Decimal};
use sc_client_api::Backend;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient,
	TestNetFactory,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::{blake2_128, Pair};
use sp_keyring::AccountKeyring;
use tokio::runtime::Runtime;
use trie_db::{TrieDBMut, TrieDBMutBuilder};

use bls_primitives::Pair as BLSPair;
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{ObMessage, StateSyncStatus, WithdrawPayloadCallByUser, WithdrawalRequest},
	ObApi, SnapshotSummary, ValidatorSet,
};
use polkadex_primitives::{ingress::IngressMessages, AccountId, AssetId, BlockNumber};

use crate::worker::{ObWorker, WorkerParams};
use parking_lot::RwLock;
use sp_runtime::SaturatedConversion;

pub(crate) fn make_ob_ids(keys: &[AccountKeyring]) -> Vec<AuthorityId> {
	SnapshotSummary::default();
	keys.iter()
		.map(|key| {
			let seed = key.to_seed();
			println!("Loaded seed: {}", seed);
			BLSPair::from_string(&seed, None).unwrap().public().into()
		})
		.collect()
}

macro_rules! create_test_api {
    ( $api_name:ident, latest_summary: $latest_summary:expr,ingress_messages: $ingress_messages:expr, $($inits:expr),+ ) => {
		pub(crate) mod $api_name {
			use super::*;

			#[derive(Clone, Default)]
			pub(crate) struct TestApi {}

			// compiler gets confused and warns us about unused inner
			#[allow(dead_code)]
			pub(crate) struct RuntimeApi {
				inner: TestApi,
			}

			impl ProvideRuntimeApi<Block> for TestApi {
				type Api = RuntimeApi;
				fn runtime_api<'a>(&'a self) -> ApiRef<'a, Self::Api> {
					RuntimeApi { inner: self.clone() }.into()
				}
			}
			sp_api::mock_impl_runtime_apis! {
                impl ObApi<Block> for RuntimeApi {
                    /// Return the current active Orderbook validator set
					fn validator_set() -> ValidatorSet<AuthorityId>{ValidatorSet::new(make_ob_ids(&[$($inits),+]), 0).unwrap()}

					fn get_latest_snapshot() -> SnapshotSummary{$latest_summary}

					/// Return the ingress messages at the given block
					fn ingress_messages() -> Vec<polkadex_primitives::ingress::IngressMessages<AccountId>>{$ingress_messages}

					/// Submits the snapshot to runtime
					fn submit_snapshot(_: SnapshotSummary) -> Result<(), ()>{Ok(())}

					/// Get Snapshot By Id
					fn get_snapshot_by_id(_: u64) -> Option<SnapshotSummary>{Some($latest_summary)}

					/// Returns all main account and corresponding proxies at this point in time
					fn get_all_accounts_and_proxies() -> Vec<(AccountId,Vec<AccountId>)>{Vec::new()}

					/// Returns snapshot generation intervals
					fn get_snapshot_generation_intervals() -> (u64,BlockNumber){(0,0)}
                }
			}
		}
	};
}

create_test_api!(
	two_validators,
	latest_summary: SnapshotSummary::default(),
	ingress_messages: vec![],
	AccountKeyring::Alice,
	AccountKeyring::Bob
);

#[derive(Default)]
pub struct PeerData {
	is_validator: bool,
	peer_rpc_link: Option<UnboundedSender<ObMessage>>,
}

#[derive(Default)]
pub struct ObTestnet {
	peers: Vec<Peer<PeerData, PeersClient>>,
}

impl TestNetFactory for ObTestnet {
	type Verifier = PassThroughVerifier;
	type BlockImport = PeersClient;
	type PeerData = PeerData;

	fn make_verifier(&self, _: PeersClient, _: &Self::PeerData) -> Self::Verifier {
		PassThroughVerifier::new(true) // we don't care about how blks are finalized
	}

	fn peer(&mut self, i: usize) -> &mut Peer<PeerData, PeersClient> {
		&mut self.peers[i]
	}

	fn peers(&self) -> &Vec<Peer<PeerData, PeersClient>> {
		&self.peers
	}

	fn mut_peers<F: FnOnce(&mut Vec<Peer<PeerData, PeersClient>>)>(&mut self, closure: F) {
		closure(&mut self.peers);
	}

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<sc_consensus::import_queue::BoxJustificationImport<sc_network_test::Block>>,
		Self::PeerData,
	) {
		(client.as_block_import(), None, PeerData { is_validator: false, peer_rpc_link: None })
	}
	fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![],
			is_authority: false,
			..Default::default()
		})
	}
}

impl ObTestnet {
	pub(crate) fn new(n_authority: usize, n_full: usize) -> Self {
		let mut net = ObTestnet { peers: Vec::with_capacity(n_authority + n_full) };
		for _ in 0..n_authority {
			net.add_authority_peer();
		}
		for _ in 0..n_full {
			net.add_full_peer();
		}
		net
	}

	pub(crate) fn add_authority_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![],
			is_authority: true,
			..Default::default()
		})
	}
}

// Spawns Orderbook worker. Returns a future to spawn on the runtime.
fn initialize_orderbook<API>(
	net: &mut ObTestnet,
	peers: Vec<(usize, &AccountKeyring, Arc<API>, bool)>,
) -> impl Future<Output = ()>
where
	API: ProvideRuntimeApi<Block> + Default + Sync + Send,
	API::Api: ObApi<Block>,
{
	let workers = FuturesUnordered::new();
	for (peer_id, key, api, is_validator) in peers.into_iter() {
		let (sender, receiver) = futures::channel::mpsc::unbounded();
		net.peers[peer_id].data.peer_rpc_link = Some(sender);
		net.peers[peer_id].data.is_validator = is_validator;

		let peer = &net.peers[peer_id];
		// Generate the crypto material with test keys
		bls_primitives::crypto::bls_ext::generate_pair_and_store(Some(
			key.to_seed().as_bytes().to_vec(),
		));

		let ob_params = crate::ObParams {
			client: peer.client().as_client(),
			backend: peer.client().as_backend(),
			runtime: api,
			key_store: None,
			network: peer.network_service().clone(),
			prometheus_registry: None,
			protocol_name: Cow::from("blah"),
			is_validator,
			message_sender_link: receiver,
			marker: Default::default(),
			last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
				0_u32.saturated_into(),
			)),
			memory_db: Arc::new(RwLock::new(MemoryDB::default())),
			working_state_root: Arc::new(RwLock::new([0; 32])),
		};
		let gadget = crate::start_orderbook_gadget::<_, _, _, _, _>(ob_params);

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&gadget);
		workers.push(gadget);
	}

	workers.for_each(|_| async move {})
}

// TODO: Make this work
// use sc_network_gossip::Network as GossipNetwork;
// pub fn setup_one<B, BE, C, SO, N, R>(api: Arc<R>, is_validator: bool) -> (ObWorker<B, BE, C, SO,
// N, R>, UnboundedSender<ObMessage>) where
//     B: BlockT,
//     BE: Backend<B>,
//     C: Client<B, BE>,
//     R: ProvideRuntimeApi<B>,
//     R::Api: ObApi<B>,
//     N: GossipNetwork<B> + Clone + Send + Sync + 'static + SyncOracle,
//
// {
//     let testnet = ObTestnet::new(1, 0);
//     let peer = &testnet.peers[0];
//     let (rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();
//
//     let worker_params = WorkerParams {
//         client: peer.client().as_client(),
//         backend: peer.client().as_backend(),
//         runtime: api,
//         sync_oracle: peer.network_service().clone(),
//         network: peer.network_service().clone(),
//         protocol_name: Cow::from("blah"),
//         is_validator,
//         message_sender_link: rpc_receiver,
//         metrics: None,
//         _marker: Default::default(),
//     };
//
//     let worker = ObWorker::new(worker_params);
//
//     (worker, rpc_sender)
// }

#[test]
pub fn test_network() {
	sp_tracing::try_init_simple();

	let runtime = Runtime::new().unwrap();
	let peers = &[(AccountKeyring::Alice, true), (AccountKeyring::Bob, true)];
	let mut net = ObTestnet::new(2, 0);

	let api = Arc::new(two_validators::TestApi {});
	let ob_peers = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, api.clone(), *is_auth))
		.collect();
	runtime.spawn(initialize_orderbook(&mut net, ob_peers));
}

#[tokio::test]
pub async fn test_single_worker() {
	let alice = AccountKeyring::Alice.pair();
	let bob = AccountKeyring::Bob.pair();
	let alice_acc = AccountId::from(alice.public());
	let bob_acc = AccountId::from(bob.public());
	// Setup runtime
	create_test_api!(
		one_validator,
		latest_summary: SnapshotSummary::default(),
		ingress_messages:
			vec![
				IngressMessages::RegisterUser(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Bob.pair().public())
				),
				IngressMessages::AddProxy(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Charlie.pair().public())
				),
				IngressMessages::Deposit(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AssetId::Polkadex,
					Decimal::from_f64(10.2).unwrap()
				)
			],
		AccountKeyring::Alice
	);
	let api = Arc::new(one_validator::TestApi {});
	// Setup worker
	let testnet = ObTestnet::new(1, 0);
	let peer = &testnet.peers[0];
	let (_rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

	let worker_params = WorkerParams {
		client: peer.client().as_client(),
		backend: peer.client().as_backend(),
		runtime: api,
		sync_oracle: peer.network_service().clone(),
		network: peer.network_service().clone(),
		protocol_name: Cow::from("blah"),
		is_validator: true,
		message_sender_link: rpc_receiver,
		metrics: None,
		_marker: Default::default(),
		last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
			0_u32.saturated_into(),
		)),
		memory_db: Arc::new(RwLock::new(MemoryDB::default())),
		working_state_root: Arc::new(RwLock::new([0; 32])),
	};

	let mut worker = ObWorker::new(worker_params);

	let payload = WithdrawPayloadCallByUser {
		asset_id: AssetId::Polkadex,
		amount: "1".to_string(),
		timestamp: 0,
	};
	let withdraw_request = WithdrawalRequest {
		signature: bob.sign(&payload.encode()).into(),
		payload: payload.clone(),
		main: alice_acc.clone(),
		proxy: bob_acc,
	};

	worker.handle_blk_import(0).unwrap();

	worker.process_withdraw(withdraw_request, 0).unwrap();
	let charlie = AccountKeyring::Charlie.pair();
	let charlie_acc = AccountId::from(charlie.public());
	let withdraw_request = WithdrawalRequest {
		signature: charlie.sign(&payload.encode()).into(),
		payload,
		main: alice_acc,
		proxy: charlie_acc,
	};
	worker.process_withdraw(withdraw_request, 0).unwrap()
}

// Setup runtime
#[tokio::test]
pub async fn test_offline_storage() {
	let alice = AccountKeyring::Alice.pair();
	let bob = AccountKeyring::Bob.pair();
	let _alice_acc = AccountId::from(alice.public());
	let _bob_acc = AccountId::from(bob.public());
	create_test_api!(
		one_validator,
		latest_summary: SnapshotSummary::default(),
		ingress_messages:
			vec![
				IngressMessages::RegisterUser(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Bob.pair().public())
				),
				IngressMessages::AddProxy(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Charlie.pair().public())
				),
				IngressMessages::Deposit(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AssetId::Polkadex,
					Decimal::from_f64(10.2).unwrap()
				)
			],
		AccountKeyring::Alice
	);
	let api = Arc::new(one_validator::TestApi {});
	// Setup worker
	let testnet = ObTestnet::new(1, 0);
	let peer = &testnet.peers[0];
	let (_rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

	let worker_params = WorkerParams {
		client: peer.client().as_client(),
		backend: peer.client().as_backend(),
		runtime: api,
		sync_oracle: peer.network_service().clone(),
		network: peer.network_service().clone(),
		protocol_name: Cow::from("blah"),
		is_validator: true,
		message_sender_link: rpc_receiver,
		metrics: None,
		_marker: Default::default(),
		last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
			0_u32.saturated_into(),
		)),
		memory_db: Arc::new(RwLock::new(MemoryDB::default())),
		working_state_root: Arc::new(RwLock::new([0; 32])),
	};
	assert!(worker_params.backend.offchain_storage().is_some());
	let _worker = ObWorker::new(worker_params);
}

#[test]
pub fn test_trie_insertion() {
	let mut working_state_root = [0u8; 32];
	let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
	{
		let mut trie: TrieDBMut<ExtensionLayout> =
			TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root).build();

		//trie.insert(b"ab".as_ref(),b"cd".as_ref()).unwrap();
		trie.commit();
	}

	// {
	//     let mut trie: TrieDBMut<ExtensionLayout> =
	//         TrieDBMutBuilder::from_existing(&mut memory_db, &mut working_state_root)
	//             .build();
	//    assert_eq!(trie.get(b"ab".as_ref()).unwrap(), Some(b"cd".to_vec()))
	// }

	assert_ne!(working_state_root, [0u8; 32]);
}

#[tokio::test]
pub async fn test_process_chunk() {
	let alice = AccountKeyring::Alice.pair();
	let bob = AccountKeyring::Bob.pair();
	let _alice_acc = AccountId::from(alice.public());
	let _bob_acc = AccountId::from(bob.public());
	let data: Vec<u8> = [1u8; 10].to_vec();
	let computed_hash: H128 = H128::from(blake2_128(&data));
	let _snapshot_summary = SnapshotSummary {
		snapshot_id: 10,
		state_root: Default::default(),
		state_change_id: 0,
		state_chunk_hashes: vec![computed_hash],
		bitflags: vec![],
		withdrawals: vec![],
		aggregate_signature: None,
	};
	create_test_api!(
		one_validator,
		latest_summary: SnapshotSummary {
		snapshot_id: 10,
		state_root: Default::default(),
		state_change_id: 0,
		state_chunk_hashes: vec![H128::from(blake2_128(&[1u8;10]))],
		bitflags: vec![],
		withdrawals: vec![],
		aggregate_signature: None
	},
		ingress_messages:
			vec![
				IngressMessages::RegisterUser(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Bob.pair().public())
				),
				IngressMessages::AddProxy(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Charlie.pair().public())
				),
				IngressMessages::Deposit(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AssetId::Polkadex,
					Decimal::from_f64(10.2).unwrap()
				)
			],
		AccountKeyring::Alice
	);
	let api = Arc::new(one_validator::TestApi {});
	// Setup worker
	let testnet = ObTestnet::new(1, 0);
	let peer = &testnet.peers[0];
	let (_rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

	let worker_params = WorkerParams {
		client: peer.client().as_client(),
		backend: peer.client().as_backend(),
		runtime: api,
		network: peer.network_service().clone(),
		sync_oracle: peer.network_service().clone(),
		protocol_name: Cow::from("blah"),
		is_validator: true,
		message_sender_link: rpc_receiver,
		metrics: None,
		_marker: Default::default(),
		last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
			0_u32.saturated_into(),
		)),
		memory_db: Arc::new(RwLock::new(MemoryDB::default())),
		working_state_root: Arc::new(RwLock::new([0; 32])),
	};
	assert!(worker_params.backend.offchain_storage().is_some());
	let mut worker = ObWorker::new(worker_params);
	let snapshot_id = 10;
	let index = 0;
	worker.process_chunk(&snapshot_id, &index, &data);
	let status = worker.get_sync_state_map_value(index);
	assert_eq!(status, StateSyncStatus::Available);
}

// Test `store_snapshot` function then retrieve and decode that snapshot to verify its correctness.
#[tokio::test]
pub async fn test_store_snapshot() {
	let alice = AccountKeyring::Alice.pair();
	let bob = AccountKeyring::Bob.pair();
	let _alice_acc = AccountId::from(alice.public());
	let _bob_acc = AccountId::from(bob.public());
	create_test_api!(
		one_validator,
		latest_summary: SnapshotSummary::default(),
		ingress_messages:
			vec![
				IngressMessages::RegisterUser(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Bob.pair().public())
				),
				IngressMessages::AddProxy(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Charlie.pair().public())
				),
				IngressMessages::Deposit(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AssetId::Polkadex,
					Decimal::from_f64(10.2).unwrap()
				)
			],
		AccountKeyring::Alice
	);
	let api = Arc::new(one_validator::TestApi {});
	// Setup worker
	let testnet = ObTestnet::new(1, 0);
	let peer = &testnet.peers[0];
	let (_rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

	let worker_params = WorkerParams {
		client: peer.client().as_client(),
		backend: peer.client().as_backend(),
		runtime: api,
		network: peer.network_service().clone(),
		sync_oracle: peer.network_service().clone(),
		protocol_name: Cow::from("blah"),
		is_validator: true,
		message_sender_link: rpc_receiver,
		metrics: None,
		_marker: Default::default(),
		last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
			0_u32.saturated_into(),
		)),
		memory_db: Arc::new(RwLock::new(MemoryDB::default())),
		working_state_root: Arc::new(RwLock::new([0; 32])),
	};
	assert!(worker_params.backend.offchain_storage().is_some());
	let mut worker = ObWorker::new(worker_params);

	let snapshot_id = 3_u64;
	let state_change_id = 4_u64;

	let offline_storage_for_snapshot = worker.get_offline_storage(snapshot_id);
	assert_eq!(offline_storage_for_snapshot, None);

	let snapshot_summary = worker.store_snapshot(state_change_id, snapshot_id).unwrap();
	let offline_storage_for_snapshot = worker.get_offline_storage(snapshot_id).unwrap();
	let store_summary = SnapshotSummary::decode(&mut &offline_storage_for_snapshot[..]).unwrap();
	assert_eq!(snapshot_summary, store_summary);
}

// Test `load_snapshot` function, then retrieve and decode the last snapshot and assert if correct
// snapshot was loaded. Also assert if workers last snapshot was updated.
#[tokio::test]
pub async fn test_load_snapshot() {
	let alice = AccountKeyring::Alice.pair();
	let bob = AccountKeyring::Bob.pair();
	let _alice_acc = AccountId::from(alice.public());
	let _bob_acc = AccountId::from(bob.public());
	create_test_api!(
		one_validator,
		latest_summary: SnapshotSummary::default(),
		ingress_messages:
			vec![
				IngressMessages::RegisterUser(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Bob.pair().public())
				),
				IngressMessages::AddProxy(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Charlie.pair().public())
				),
				IngressMessages::Deposit(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AssetId::Polkadex,
					Decimal::from_f64(10.2).unwrap()
				)
			],
		AccountKeyring::Alice
	);
	let api = Arc::new(one_validator::TestApi {});
	// Setup worker
	let testnet = ObTestnet::new(1, 0);
	let peer = &testnet.peers[0];
	let (_rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

	let worker_params = WorkerParams {
		client: peer.client().as_client(),
		backend: peer.client().as_backend(),
		runtime: api,
		network: peer.network_service().clone(),
		sync_oracle: peer.network_service().clone(),
		protocol_name: Cow::from("blah"),
		is_validator: true,
		message_sender_link: rpc_receiver,
		metrics: None,
		_marker: Default::default(),
		last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
			0_u32.saturated_into(),
		)),
		memory_db: Arc::new(RwLock::new(MemoryDB::default())),
		working_state_root: Arc::new(RwLock::new([0; 32])),
	};
	assert!(worker_params.backend.offchain_storage().is_some());
	let mut worker = ObWorker::new(worker_params);

	let snapshot_id = 3_u64;
	let state_change_id = 4_u64;

	let get_snapshot_summary = worker.last_snapshot.clone();
	assert_eq!(get_snapshot_summary.read().snapshot_id, 0);
	assert_eq!(get_snapshot_summary.read().state_change_id, 0);

	let snapshot_summary = worker.store_snapshot(state_change_id, snapshot_id).unwrap();
	assert!(worker.load_snapshot(&snapshot_summary).is_ok());

	let get_snapshot_summary = worker.last_snapshot.clone();
	assert_eq!(get_snapshot_summary.read().snapshot_id, snapshot_id);
	assert_eq!(get_snapshot_summary.read().state_change_id, state_change_id);
}

// Test `load_snapshot` with invalid summary. Also assert that workers last snapshot should not be
// updated.
#[tokio::test]
pub async fn test_load_snapshot_with_invalid_summary() {
	let alice = AccountKeyring::Alice.pair();
	let bob = AccountKeyring::Bob.pair();
	let _alice_acc = AccountId::from(alice.public());
	let _bob_acc = AccountId::from(bob.public());
	create_test_api!(
		one_validator,
		latest_summary: SnapshotSummary::default(),
		ingress_messages:
			vec![
				IngressMessages::RegisterUser(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Bob.pair().public())
				),
				IngressMessages::AddProxy(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AccountId::from(AccountKeyring::Charlie.pair().public())
				),
				IngressMessages::Deposit(
					AccountId::from(AccountKeyring::Alice.pair().public()),
					AssetId::Polkadex,
					Decimal::from_f64(10.2).unwrap()
				)
			],
		AccountKeyring::Alice
	);
	let api = Arc::new(one_validator::TestApi {});
	// Setup worker
	let testnet = ObTestnet::new(1, 0);
	let peer = &testnet.peers[0];
	let (_rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

	let worker_params = WorkerParams {
		client: peer.client().as_client(),
		backend: peer.client().as_backend(),
		runtime: api,
		network: peer.network_service().clone(),
		sync_oracle: peer.network_service().clone(),
		protocol_name: Cow::from("blah"),
		is_validator: true,
		message_sender_link: rpc_receiver,
		metrics: None,
		_marker: Default::default(),
		last_successful_block_number_snapshot_created: Arc::new(RwLock::new(
			0_u32.saturated_into(),
		)),
		memory_db: Arc::new(RwLock::new(MemoryDB::default())),
		working_state_root: Arc::new(RwLock::new([0; 32])),
	};
	assert!(worker_params.backend.offchain_storage().is_some());
	let mut worker = ObWorker::new(worker_params);

	let snapshot_id = 3_u64;
	let state_change_id = 4_u64;

	let get_snapshot_summary = worker.last_snapshot.clone();
	assert_eq!(get_snapshot_summary.read().snapshot_id, 0);
	assert_eq!(get_snapshot_summary.read().state_change_id, 0);

	let mut snapshot_summary = worker.store_snapshot(state_change_id, snapshot_id).unwrap();
	snapshot_summary.state_chunk_hashes = vec![H128::random(), H128::random()];
	assert!(worker.load_snapshot(&snapshot_summary).is_err());

	let get_snapshot_summary = worker.last_snapshot.clone();
	assert_eq!(get_snapshot_summary.read().snapshot_id, 0);
	assert_eq!(get_snapshot_summary.read().state_change_id, 0);
}
