use futures::{stream::FuturesUnordered, StreamExt};
use parity_scale_codec::Encode;
use parking_lot::RwLock;
use sc_keystore::LocalKeystore;
use std::{collections::BTreeMap, future::Future, sync::Arc};

use polkadex_primitives::utils::return_set_bits;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient,
	TestNetFactory,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::Pair;
use sp_keyring::AccountKeyring;
use sp_keystore::CryptoStore;

use thea_primitives::{
	AuthorityId, AuthoritySignature, Message, Network, TheaApi, ValidatorSet, ValidatorSetId,
};

pub mod withdrawal;

#[derive(Clone, Default)]
pub(crate) struct TestApi {
	authorities: BTreeMap<Network, ValidatorSet<AuthorityId>>,
	validator_set_id: ValidatorSetId,
	next_authorities: BTreeMap<Network, ValidatorSet<AuthorityId>>,
	network_pref: BTreeMap<AuthorityId, Network>,
	outgoing_messages: BTreeMap<(Network, u64), Message>,
	incoming_messages: Arc<RwLock<BTreeMap<(Network, u64), Message>>>,
	incoming_nonce: Arc<RwLock<BTreeMap<Network, u64>>>,
	outgoing_nonce: BTreeMap<Network, u64>,
}

impl TestApi {
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
		assert_eq!(last_nonce.saturating_add(1), message.nonce);

		// Find who all signed this payload
		let signed_auths_indexes: Vec<usize> = return_set_bits(&bitmap);

		// Create a vector of public keys of everyone who signed
		let auths = self.authorities.get(&message.network).unwrap().validators.clone();
		let mut signatories: Vec<bls_primitives::Public> = vec![];
		for index in signed_auths_indexes {
			signatories.push((*auths.get(index).unwrap()).clone().into());
		}

		// Check signature
		assert!(bls_primitives::crypto::verify_aggregate_(
			&signatories[..],
			&message.encode(),
			&signature.into(),
		));

		self.incoming_nonce.write().insert(message.network, message.nonce);
		self.incoming_messages.write().insert((message.network, message.nonce), message);
		Ok(())
	}

	fn get_last_processed_nonce(&self, network: Network) -> u64 {
		*self.incoming_nonce.read().get(&network).unwrap_or(&0)
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
}

/// Helper function to convert keyring types to AuthorityId
pub(crate) fn make_ob_ids(keys: &[AccountKeyring]) -> Vec<AuthorityId> {
	keys.iter()
		.map(|key| {
			let seed = key.to_seed();
			thea_primitives::crypto::Pair::from_string(&seed, None).unwrap().public().into()
		})
		.collect()
}

#[derive(Default)]
pub struct PeerData {
	is_validator: bool,
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
		(client.as_block_import(), None, PeerData { is_validator: false })
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
			net.add_full_peer_with_config(FullPeerConfig {
				notifications_protocols: vec![crate::thea_protocol_name::NAME.into()],
				is_authority: false,
				..Default::default()
			});
		}
		net
	}

	pub(crate) fn add_authority_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![crate::thea_protocol_name::NAME.into()],
			is_authority: true,
			..Default::default()
		})
	}
}

// Spawns Orderbook worker. Returns a future to spawn on the runtime.
async fn initialize_orderbook<API>(
	net: &mut ObTestnet,
	peers: Vec<(usize, &AccountKeyring, Arc<API>, bool)>,
) -> impl Future<Output = ()>
where
	API: ProvideRuntimeApi<Block> + Default + Sync + Send,
	API::Api: TheaApi<Block>,
{
	let workers = FuturesUnordered::new();
	for (peer_id, key, api, is_validator) in peers.into_iter() {
		net.peers[peer_id].data.is_validator = is_validator;

		let mut keystore = None;

		if is_validator {
			// Generate the crypto material with test keys,
			// we have to use file based keystore,
			// in memory keystore doesn't seem to work here
			keystore = Some(Arc::new(
				LocalKeystore::open(format!("keystore-{:?}", peer_id), None).unwrap(),
			));
			let (pair, seed) =
				thea_primitives::crypto::Pair::from_string_with_seed(&key.to_seed(), None).unwrap();
			// Insert the key
			keystore
				.as_ref()
				.unwrap()
				.insert_unknown(thea_primitives::KEY_TYPE, &key.to_seed(), pair.public().as_ref())
				.await
				.unwrap();
			// Check if the key is present or not
			keystore
				.as_ref()
				.unwrap()
				.key_pair::<thea_primitives::crypto::Pair>(&pair.public())
				.unwrap();
		}

		let thea_params = crate::TheaParams {
			client: net.peers[peer_id].client().as_client(),
			backend: net.peers[peer_id].client().as_backend(),
			runtime: api,
			keystore,
			network: net.peers[peer_id].network_service().clone(),
			prometheus_registry: None,
			protocol_name: crate::thea_protocol_name::NAME.into(),
			is_validator,
			marker: Default::default(),
		};
		let gadget = crate::start_thea_gadget::<_, _, _, _, _>(thea_params);

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&gadget);
		workers.push(gadget);
	}

	workers.for_each(|_| async move {})
}

pub async fn generate_and_finalize_blocks(
	count: usize,
	testnet: &mut ObTestnet,
	peer_index: usize,
) {
	let old_finalized = testnet.peer(peer_index).client().info().finalized_number;
	testnet.peer(peer_index).push_blocks(count, false);
	// wait for blocks to propagate
	testnet.run_until_sync().await; // It should be run_until_sync() for finality to work properly.

	assert_eq!(
		old_finalized + count as u64,
		testnet.peer(peer_index).client().info().finalized_number
	);
}
