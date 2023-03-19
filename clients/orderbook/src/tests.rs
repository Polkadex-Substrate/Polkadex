use std::borrow::Cow;
use std::future::Future;
use std::sync::Arc;

use futures::channel::mpsc::UnboundedSender;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use log::trace;
use memory_db::{HashKey, MemoryDB};
use parity_scale_codec::Encode;
use reference_trie::{ExtensionLayout, RefHasher};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use sc_client_api::{Backend, BlockchainEvents};
use sc_network::config::{build_multiaddr, EmptyTransactionPool, Role};
use sc_network::NetworkWorker;
use sc_network_common::service::NetworkStateInfo;
use sc_network_gossip::MessageIntent::PeriodicRebroadcast;
use sc_network_test::{Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer,
                      PeersClient, TestClientBuilder, TestClientBuilderExt, TestNetFactory};
use sp_api::{ApiRef, BlockT, ProvideRuntimeApi};
use sp_consensus::SyncOracle;
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use tokio::runtime::Runtime;
use trie_db::{TrieDBMut, TrieDBMutBuilder, TrieMut};

use bls_primitives::Pair as BLSPair;
use orderbook_primitives::crypto::AuthorityId;
use orderbook_primitives::ObApi;
use orderbook_primitives::SnapshotSummary;
use orderbook_primitives::types::{ObMessage, UserActions, WithdrawalRequest, WithdrawPayloadCallByUser};
use orderbook_primitives::ValidatorSet;
use polkadex_primitives::{AccountId, AssetId};
use polkadex_primitives::ingress::IngressMessages;
use crate::Client;

use crate::worker::{ObWorker, ORDERBOOK_STATE_SYNC_REQUEST, ORDERBOOK_STATE_SYNC_RESPONSE, STID_IMPORT_REQUEST, STID_IMPORT_RESPONSE, WorkerParams};

pub(crate) fn make_ob_ids(keys: &[AccountKeyring]) -> Vec<AuthorityId> {
    SnapshotSummary::default();
    keys.iter().map(|key| {
        let seed = key.to_seed();
        println!("Loaded seed: {}", seed);
        BLSPair::from_string(&seed, None).unwrap().public().into()
    }).collect()
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

    fn make_block_import(&self, client: PeersClient) -> (
        BlockImportAdapter<Self::BlockImport>,
        Option<sc_consensus::import_queue::BoxJustificationImport<sc_network_test::Block>>,
        Self::PeerData) {
        (client.as_block_import(), None, PeerData { is_validator: false, peer_rpc_link: None })
    }
    fn add_full_peer(&mut self) {
        self.add_full_peer_with_config(FullPeerConfig {
            notifications_protocols: vec![
                Cow::from(ORDERBOOK_STATE_SYNC_RESPONSE),
                Cow::from(STID_IMPORT_REQUEST),
                Cow::from(STID_IMPORT_RESPONSE),
                Cow::from(ORDERBOOK_STATE_SYNC_REQUEST)],
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
            notifications_protocols: vec![
                Cow::from(ORDERBOOK_STATE_SYNC_RESPONSE),
                Cow::from(STID_IMPORT_REQUEST),
                Cow::from(STID_IMPORT_RESPONSE),
                Cow::from(ORDERBOOK_STATE_SYNC_REQUEST)],
            is_authority: true,
            ..Default::default()
        })
    }
}

// Spawns Orderbook worker. Returns a future to spawn on the runtime.
fn initialize_orderbook<API>(
    net: &mut ObTestnet,
    peers: Vec<(usize, &AccountKeyring, Arc<API>, bool)>,
) -> impl Future<Output=()>
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
        bls_primitives::crypto::bls_ext::generate_pair_and_store(Some(key.to_seed().as_bytes().to_vec()));

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
// pub fn setup_one<B, BE, C, SO, N, R>(api: Arc<R>, is_validator: bool) -> (ObWorker<B, BE, C, SO, N, R>, UnboundedSender<ObMessage>)
// where
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

    let mut runtime = Runtime::new().unwrap();
    let peers = &[(AccountKeyring::Alice, true), (AccountKeyring::Bob, true)];
    let mut net = ObTestnet::new(2, 0);

    let api = Arc::new(two_validators::TestApi {});
    let ob_peers = peers.iter().enumerate().map(|(id, (key, is_auth))| (id, key, api.clone(), *is_auth)).collect();
    runtime.spawn(initialize_orderbook(&mut net, ob_peers));
}


#[tokio::test]
pub async fn test_single_worker_process_withdraw() {
    let alice = AccountKeyring::Alice.pair();
    let bob = AccountKeyring::Bob.pair();
    let alice_acc =  AccountId::from(alice.public());
    let bob_acc = AccountId::from(bob.public());
    let ingress_messages = vec![
        IngressMessages::RegisterUser(alice_acc.clone(),bob_acc.clone()),
        IngressMessages::Deposit(alice_acc.clone(),AssetId::polkadex,Decimal::from_f64(10.2).unwrap())
    ];
    // Setup runtime
    create_test_api!(
	one_validator,
	latest_summary: SnapshotSummary::default(),
	ingress_messages: vec![],
	AccountKeyring::Alice);
    let api = Arc::new(one_validator::TestApi {});
    // Setup worker
    let testnet = ObTestnet::new(1, 0);
    let peer = &testnet.peers[0];
    let (rpc_sender, rpc_receiver) = futures::channel::mpsc::unbounded();

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
    };

    let mut worker = ObWorker::new(worker_params);
    let payload = WithdrawPayloadCallByUser {
        asset_id: AssetId::polkadex,
        amount: "10".to_string(),
        timestamp: 0,
    };
    let withdraw_request = WithdrawalRequest{
        signature: bob.sign(&payload.encode()).into(),
        payload,
        main: alice_acc,
        proxy: bob_acc,
    };
    worker.handle_blk_import(0).unwrap();

    assert_ne!(worker.working_state_root, [0u8; 32]);
    worker.process_withdraw(withdraw_request).unwrap()
}


#[test]
pub fn test_trie_insertion() {
    let mut working_state_root = [0u8;32];
    let mut memory_db: MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>> = Default::default();
    {
        let mut trie: TrieDBMut<ExtensionLayout> =
            TrieDBMutBuilder::new(&mut memory_db, &mut working_state_root)
                .build();

        trie.insert(b"ab".as_ref(),b"cd".as_ref()).unwrap();
        trie.commit();
    }

    {
        let mut trie: TrieDBMut<ExtensionLayout> =
            TrieDBMutBuilder::from_existing(&mut memory_db, &mut working_state_root)
                .build();
        assert_eq!(trie.get(b"ab".as_ref()).unwrap(), Some(b"cd".to_vec()))
    }

    assert_ne!(working_state_root, [0u8; 32]);
}