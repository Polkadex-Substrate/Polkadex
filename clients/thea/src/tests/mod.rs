use crate::{connector::traits::ForeignConnector, types::GossipMessage, worker::TheaWorker};
use futures::{
	stream::{Fuse, FuturesUnordered},
	StreamExt,
};
use parity_scale_codec::Encode;
use parking_lot::RwLock;
use polkadex_primitives::utils::return_set_bits;
use sc_client_api::{BlockchainEvents, FinalityNotification};
use sc_consensus::LongestChain;
use sc_finality_grandpa::{
	block_import, run_grandpa_voter, Config, GenesisAuthoritySetProvider, GrandpaParams, LinkHalf,
	SharedVoterState,
};
use sc_keystore::LocalKeystore;
use sc_network::{config::Role, NetworkService};
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, Hash, PassThroughVerifier, Peer, PeersClient,
	PeersFullClient, TestNetFactory,
};
use sc_utils::mpsc::TracingUnboundedReceiver;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::{Pair, H256};
use sp_finality_grandpa::{
	AuthorityList, EquivocationProof, GrandpaApi, OpaqueKeyOwnershipProof, SetId,
};
use sp_keyring::AccountKeyring;
use sp_keystore::{SyncCryptoStore, SyncCryptoStorePtr};
use sp_runtime::key_types::GRANDPA;
use std::{
	collections::{BTreeMap, HashMap},
	future::Future,
	sync::{Arc, Mutex},
	time::Duration,
};
use substrate_test_runtime_client::Ed25519Keyring;
use thea_primitives::{
	AuthorityId, AuthoritySignature, Message, Network, TheaApi, ValidatorSet, ValidatorSetId,
};
use tokio::time::Instant;

//pub mod deposit;
mod grandpa;
//mod protocol;
//pub mod withdrawal;

pub(crate) use grandpa::*;

#[derive(Clone, Default)]
// This is the mock of native runtime state
pub(crate) struct TestApi {
	genesys_authorities: AuthorityList,
	authorities: BTreeMap<Network, ValidatorSet<AuthorityId>>,
	validator_set_id: ValidatorSetId,
	_next_authorities: BTreeMap<Network, ValidatorSet<AuthorityId>>,
	network_pref: BTreeMap<AuthorityId, Network>,
	outgoing_messages: BTreeMap<(Network, u64), Message>,
	incoming_messages: Arc<RwLock<BTreeMap<(Network, u64), Message>>>,
	incoming_nonce: Arc<RwLock<BTreeMap<Network, u64>>>,
	_outgoing_nonce: BTreeMap<Network, u64>,
}

impl TestApi {
	fn full_validator_set(&self) -> Option<ValidatorSet<AuthorityId>> {
		let mut full_list = vec![];
		for list in self.authorities.values() {
			full_list.append(&mut list.validators.clone())
		}
		ValidatorSet::new(full_list, self.validator_set_id)
	}

	fn validator_set(&self, network: Network) -> Option<ValidatorSet<AuthorityId>> {
		self.authorities.get(&network).cloned()
	}

	fn outgoing_messages(&self, network: Network, nonce: u64) -> Option<Message> {
		self.outgoing_messages.get(&(network, nonce)).cloned()
	}

	fn network(&self, auth: AuthorityId) -> Option<Network> {
		self.network_pref.get(&auth).cloned()
	}

	fn incoming_message(
		&self,
		message: Message,
		bitmap: Vec<u128>,
		signature: AuthoritySignature,
	) -> Result<(), ()> {
		let last_nonce = self.incoming_nonce.read().get(&message.network).unwrap_or(&0).clone();
		if last_nonce.saturating_add(1) != message.nonce {
			return Ok(()) // Don't throw error here to mimic the behaviour of transaction
			  // pool which ignores the the transaction if the nonce is wrong.
		}

		// Find who all signed this payload
		let signed_auths_indexes: Vec<usize> = return_set_bits(&bitmap);

		// Create a vector of public keys of everyone who signed
		let auths = self.authorities.get(&message.network).unwrap().validators.clone();
		let mut signatories: Vec<bls_primitives::Public> = vec![];
		for index in signed_auths_indexes {
			signatories.push((*auths.get(index).unwrap()).clone().into());
		}

		let bls_signature: bls_primitives::Signature = signature.into();
		// Check signature
		assert!(bls_signature.verify(&signatories, &message.encode()));

		self.incoming_nonce.write().insert(message.network, message.nonce);
		self.incoming_messages.write().insert((message.network, message.nonce), message);
		Ok(())
	}

	fn get_last_processed_nonce(&self, network: Network) -> u64 {
		assert_ne!(network, 0); // don't ask for native network here.
		*self.incoming_nonce.read().get(&network).unwrap_or(&0)
	}
}

impl GenesisAuthoritySetProvider<Block> for TestApi {
	fn get(&self) -> sp_blockchain::Result<AuthorityList> {
		Ok(self.genesys_authorities.clone())
	}
}

// compiler gets confused and warns us about unused inner
#[allow(dead_code)]
pub(crate) struct RuntimeApi {
	inner: TestApi,
}

impl ProvideRuntimeApi<Block> for TestApi {
	type Api = RuntimeApi;
	fn runtime_api(&self) -> ApiRef<Self::Api> {
		RuntimeApi { inner: self.clone() }.into()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl TheaApi<Block> for RuntimeApi {
		/// Return the current active Thea validator set for all networks
		fn full_validator_set() -> Option<ValidatorSet<AuthorityId>>{
			self.inner.full_validator_set()
		}

		   /// Return the current active Thea validator set
		fn validator_set(network: Network) -> Option<ValidatorSet<AuthorityId>>{
			self.inner.validator_set(network)
		}

		/// Returns the outgoing message for given network and blk
		fn outgoing_messages(network: Network, nonce: u64) -> Option<Message>{
			self.inner.outgoing_messages(network,nonce)
		}

		/// Get Thea network associated with Validator
		fn network(auth: AuthorityId) -> Option<Network>{
			self.inner.network(auth)
		}

		/// Incoming messages
		fn incoming_message(message: Message, bitmap: Vec<u128>, signature: AuthoritySignature) -> Result<(),()>{
			self.inner.incoming_message(message, bitmap, signature)
		}

		/// Get last processed nonce for a given network
		fn get_last_processed_nonce(network: Network) -> u64{
			self.inner.get_last_processed_nonce(network)
		}
	}

	impl GrandpaApi<Block> for RuntimeApi {
		fn grandpa_authorities(&self) -> AuthorityList {
			self.inner.genesys_authorities.clone()
		}

		fn current_set_id(&self) -> SetId {
				0
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: EquivocationProof<Hash, GrandpaBlockNumber>,
			_key_owner_proof: OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: SetId,
			_authority_id: sp_finality_grandpa::AuthorityId,
		) -> Option<OpaqueKeyOwnershipProof> {
			None
		}
	}
}

/// Helper function to convert keyring types to AuthorityId
pub(crate) fn make_thea_ids(keys: &[AccountKeyring]) -> Vec<AuthorityId> {
	keys.iter()
		.map(|key| {
			let seed = key.to_seed();
			thea_primitives::crypto::Pair::from_string(&seed, None).unwrap().public().into()
		})
		.collect()
}

#[derive(Default)]
pub struct PeerData {
	_is_validator: bool,
}

#[derive(Default)]
pub struct TheaTestnet {
	api: Arc<TestApi>,
	peers: Vec<GrandpaPeer>,
	worker_massages: HashMap<usize, Arc<RwLock<BTreeMap<Message, (Instant, GossipMessage)>>>>,
}

impl TheaTestnet {
	pub(crate) fn new(n_authority: usize, n_full: usize, api: Arc<TestApi>) -> Self {
		let mut net = TheaTestnet {
			api,
			peers: Vec::with_capacity(n_authority + n_full),
			worker_massages: HashMap::new(),
		};
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
				GRANDPA_PROTOCOL_NAME.into(),
				crate::protocol_standard_name(),
			],
			is_authority: true,
			..Default::default()
		})
	}

	pub(crate) fn drop_validator(&mut self) {
		drop(self.peers.remove(0))
	}
}

impl TestNetFactory for TheaTestnet {
	type Verifier = PassThroughVerifier;
	type BlockImport = GrandpaBlockImport;
	type PeerData = GrandpaPeerData;

	fn make_verifier(&self, _: PeersClient, _: &Self::PeerData) -> Self::Verifier {
		PassThroughVerifier::new(false)
	}

	fn peer(&mut self, i: usize) -> &mut GrandpaPeer {
		&mut self.peers[i]
	}

	fn peers(&self) -> &Vec<GrandpaPeer> {
		&self.peers
	}

	fn mut_peers<F: FnOnce(&mut Vec<GrandpaPeer>)>(&mut self, closure: F) {
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
		//(client.as_block_import(), None, PeerData { is_validator: false })
		let (client, backend) = (client.as_client(), client.as_backend());
		let (import, link) =
			block_import(client, self.api.as_ref(), LongestChain::new(backend), None)
				.expect("Could not create block import for fresh peer.");
		let justification_import = Box::new(import.clone());
		(BlockImportAdapter::new(import), Some(justification_import), Mutex::new(Some(link)))
	}

	fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![
				GRANDPA_PROTOCOL_NAME.into(),
				crate::protocol_standard_name(),
			],
			is_authority: false,
			..Default::default()
		})
	}
}

/// Spawns Thea worker. Returns a future to spawn on the runtime.
pub(crate) async fn initialize_thea<API, FC>(
	net: &mut TheaTestnet,
	peers: Vec<(usize, &AccountKeyring, Arc<API>, bool, Arc<FC>)>,
) -> impl Future<Output = ()>
where
	API: ProvideRuntimeApi<Block> + Default + Sync + Send,
	API::Api: TheaApi<Block>,
	FC: ForeignConnector,
{
	let workers = FuturesUnordered::new();
	for (peer_id, key, api, is_validator, connector) in peers.into_iter() {
		let mut keystore = None;

		if is_validator {
			// Generate the crypto material with test keys,
			// we have to use file based keystore,
			// in memory keystore doesn't seem to work here
			keystore = Some(Arc::new(
				LocalKeystore::open(format!("keystore-{:?}", peer_id), None).unwrap(),
			));
			let (pair, _seed) =
				thea_primitives::crypto::Pair::from_string_with_seed(&key.to_seed(), None).unwrap();
			// Insert the key
			keystore
				.as_ref()
				.unwrap()
				.insert_unknown(thea_primitives::KEY_TYPE, &key.to_seed(), pair.public().as_ref())
				.unwrap();
			// Check if the key is present or not
			keystore
				.as_ref()
				.unwrap()
				.key_pair::<thea_primitives::crypto::Pair>(&pair.public())
				.unwrap();
		}

		let worker_params = crate::worker::WorkerParams {
			client: net.peers[peer_id].client().as_client(),
			backend: net.peers[peer_id].client().as_backend(),
			runtime: api,
			sync_oracle: net.peers[peer_id].network_service().clone(),
			keystore,
			network: net.peers[peer_id].network_service().clone(),
			_marker: Default::default(),
			is_validator,
			metrics: None,
			foreign_chain: connector,
		};
		let mut gadget = crate::worker::TheaWorker::new(worker_params).await;
		gadget.thea_network = Some(1);
		net.worker_massages.insert(peer_id, gadget.message_cache.clone());
		let run_future = gadget.run();
		fn assert_send<T: Send>(_: &T) {}
		assert_send(&run_future);
		workers.push(run_future);
	}

	workers.for_each(|_| async move {})
}

async fn create_workers_array<R, FC>(
	net: &mut TheaTestnet,
	peers: Vec<(usize, &AccountKeyring, Arc<R>, bool, Arc<FC>)>,
) -> Vec<(
	TheaWorker<
		Block,
		substrate_test_runtime_client::Backend,
		PeersFullClient,
		Arc<NetworkService<Block, H256>>,
		Arc<NetworkService<Block, H256>>,
		R,
		FC,
	>,
	Fuse<TracingUnboundedReceiver<FinalityNotification<Block>>>,
)>
where
	R: ProvideRuntimeApi<Block> + Default + Sync + Send,
	R::Api: TheaApi<Block>,
	FC: ForeignConnector,
{
	let mut workers = Vec::new();
	for (peer_id, key, api, is_validator, connector) in peers.into_iter() {
		let mut keystore = None;

		if is_validator {
			// Generate the crypto material with test keys,
			// we have to use file based keystore,
			// in memory keystore doesn't seem to work here
			keystore = Some(Arc::new(
				LocalKeystore::open(format!("keystore-{:?}", peer_id), None).unwrap(),
			));
			let (pair, _seed) =
				thea_primitives::crypto::Pair::from_string_with_seed(&key.to_seed(), None).unwrap();
			// Insert the key
			keystore
				.as_ref()
				.unwrap()
				.insert_unknown(thea_primitives::KEY_TYPE, &key.to_seed(), pair.public().as_ref())
				.unwrap();
			// Check if the key is present or not
			keystore
				.as_ref()
				.unwrap()
				.key_pair::<thea_primitives::crypto::Pair>(&pair.public())
				.unwrap();
		}

		let worker_params = crate::worker::WorkerParams {
			client: net.peers[peer_id].client().as_client(),
			backend: net.peers[peer_id].client().as_backend(),
			runtime: api,
			sync_oracle: net.peers[peer_id].network_service().clone(),
			keystore,
			network: net.peers[peer_id].network_service().clone(),
			_marker: Default::default(),
			is_validator,
			metrics: None,
			foreign_chain: connector,
		};
		let gadget = crate::worker::TheaWorker::new(worker_params).await;
		let finality_stream_future =
			net.peers[peer_id].client().as_client().finality_notification_stream().fuse();
		workers.push((gadget, finality_stream_future))
	}
	workers
}

pub async fn generate_and_finalize_blocks(count: usize, testnet: &mut TheaTestnet) {
	let fullnode_id = testnet.peers().len() - 1;
	let old_finalized = testnet.peer(fullnode_id).client().info().finalized_number;
	testnet.peer(fullnode_id).push_blocks(count, false);
	// wait for blocks to propagate
	testnet.run_until_sync().await; // It should be run_until_sync() for finality to work properly.

	assert_eq!(
		old_finalized + count as u64,
		testnet.peer(fullnode_id).client().info().finalized_number
	);
}
