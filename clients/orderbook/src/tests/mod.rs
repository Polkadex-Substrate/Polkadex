mod gosssip;
pub mod rpc;
pub mod sync;

use futures::{channel::mpsc::UnboundedSender, stream::FuturesUnordered, StreamExt};
use memory_db::{HashKey, MemoryDB};
use orderbook_primitives::{
	crypto::AuthorityId,
	types::{ObMessage, TradingPair},
	ObApi, SnapshotSummary, ValidatorSet,
};
use parking_lot::RwLock;
use polkadex_primitives::{
	ingress::IngressMessages, ocex::TradingPairConfig, withdrawal::Withdrawal, AccountId, AssetId,
	BlockNumber,
};
use primitive_types::H256;
use reference_trie::RefHasher;
use sc_keystore::LocalKeystore;
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, PassThroughVerifier, Peer, PeersClient,
	TestNetFactory,
};
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_application_crypto::RuntimeAppPublic;
use sp_arithmetic::traits::SaturatedConversion;

use sp_blockchain::{BlockStatus, HeaderBackend, Info};
use sp_core::{ecdsa::Public, Pair};
use sp_keyring::AccountKeyring;
use sp_keystore::CryptoStore;
use sp_runtime::traits::{Header, NumberFor};
use std::{collections::HashMap, future::Future, sync::Arc};

#[derive(Clone, Default)]
pub(crate) struct TestApi {
	pub active: Vec<AuthorityId>,
	pub latest_snapshot_nonce: Arc<RwLock<u64>>,
	pub snapshots: Arc<RwLock<HashMap<u64, SnapshotSummary<AccountId>>>>,
	pub unprocessed: Arc<RwLock<HashMap<(u64, H256), SnapshotSummary<AccountId>>>>,
	pub main_to_proxy_mapping: HashMap<AccountId, Vec<AccountId>>,
	pub pending_snapshot: Option<u64>,
	pub operator_key: Option<Public>,
	pub trading_config: Vec<(TradingPair, TradingPairConfig)>,
	pub withdrawals: Arc<RwLock<HashMap<u64, Vec<Withdrawal<AccountId>>>>>,
	pub ingress_messages: Vec<IngressMessages<AccountId>>,
	pub allowlisted_assets: Vec<AssetId>,
}

impl TestApi {
	pub fn validator_set(&self) -> ValidatorSet<AuthorityId> {
		ValidatorSet { set_id: 0, validators: self.active.clone() }
	}

	pub fn get_latest_snapshot(&self) -> SnapshotSummary<AccountId> {
		self.snapshots
			.read()
			.get(&*self.latest_snapshot_nonce.read())
			.unwrap_or(&SnapshotSummary {
				validator_set_id: 0,
				worker_nonce: 0,
				snapshot_id: 0,
				state_root: Default::default(),
				state_change_id: 0,
				last_processed_blk: 0,
				state_chunk_hashes: vec![],
				bitflags: vec![],
				withdrawals: vec![],
				aggregate_signature: None,
			})
			.clone()
	}

	pub fn submit_snapshot(&self, snapshot: SnapshotSummary<AccountId>) -> Result<(), ()> {
		assert_eq!(self.latest_snapshot_nonce.read().saturating_add(1), snapshot.snapshot_id);
		let summary_hash = H256::from_slice(&snapshot.sign_data());
		let working_summary =
			match self.unprocessed.read().get(&(snapshot.snapshot_id, summary_hash)).cloned() {
				None => snapshot,
				Some(mut stored_summary) => {
					let signature = snapshot.aggregate_signature.unwrap();
					let auth_index = snapshot.signed_auth_indexes().first().unwrap().clone();
					// Verify the auth signature.
					let signer: &AuthorityId = self.active.get(auth_index as usize).unwrap();
					assert!(signer.verify(&snapshot.sign_data(), &signature.into()));
					// Aggregate signature
					assert!(stored_summary.add_signature(signature).is_ok());
					// update the bitfield
					stored_summary.add_auth_index(auth_index);
					stored_summary.clone()
				},
			};

		let total_validators = self.active.len();
		if working_summary.signed_auth_indexes().len() >=
			total_validators.saturating_mul(2).saturating_div(3)
		{
			self.unprocessed.write().remove(&(working_summary.snapshot_id, summary_hash));
			let withdrawals = working_summary.withdrawals.clone();
			let mut withdrawals_map = self.withdrawals.write();
			withdrawals_map.insert(working_summary.snapshot_id, withdrawals);
			*self.latest_snapshot_nonce.write() = working_summary.snapshot_id;
			let mut snapshots = self.snapshots.write();
			snapshots.insert(working_summary.snapshot_id, working_summary);
		} else {
			let mut unprocessed = self.unprocessed.write();
			unprocessed.insert((working_summary.snapshot_id, summary_hash), working_summary);
		}

		Ok(())
	}

	pub fn get_all_accounts_and_proxies(&self) -> Vec<(AccountId, Vec<AccountId>)> {
		self.main_to_proxy_mapping.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
	}

	pub fn get_snapshot_generation_intervals(&self) -> (u64, u32) {
		(20, 5)
	}

	pub fn pending_snapshot(&self) -> Option<u64> {
		self.pending_snapshot
	}

	pub fn get_orderbook_opearator_key(&self) -> Option<Public> {
		self.operator_key
	}

	pub fn get_last_accepted_worker_nonce(&self) -> u64 {
		self.snapshots
			.read()
			.get(&*self.latest_snapshot_nonce.read())
			.unwrap_or(&SnapshotSummary {
				validator_set_id: 0,
				worker_nonce: 0,
				snapshot_id: 0,
				state_root: Default::default(),
				state_change_id: 0,
				last_processed_blk: 0,
				state_chunk_hashes: vec![],
				bitflags: vec![],
				withdrawals: vec![],
				aggregate_signature: None,
			})
			.worker_nonce
	}

	pub fn read_trading_pair_configs(&self) -> Vec<(TradingPair, TradingPairConfig)> {
		self.trading_config.clone()
	}

	pub fn get_ingress_messages(&self) -> Vec<IngressMessages<AccountId>> {
		self.ingress_messages.clone()
	}

	pub fn get_allowlisted_assets(&self) -> Vec<AssetId> {
		self.allowlisted_assets.clone()
	}
}

sp_api::mock_impl_runtime_apis! {
	impl ObApi<Block> for RuntimeApi {
		/// Return the current active Orderbook validator set
		fn validator_set() -> ValidatorSet<AuthorityId>
		{
			self.inner.validator_set()
		}

		fn get_latest_snapshot() -> SnapshotSummary<AccountId> {
			self.inner.get_latest_snapshot()
		}

		/// Return the ingress messages at the given block
		fn ingress_messages(blk: polkadex_primitives::BlockNumber) -> Vec<polkadex_primitives::ingress::IngressMessages<AccountId>> { self.inner.get_ingress_messages() }

		/// Submits the snapshot to runtime
		fn submit_snapshot(summary: SnapshotSummary<AccountId>) -> Result<(), ()> {
			self.inner.submit_snapshot(summary)
		}

		/// Get Snapshot By Id
		fn get_snapshot_by_id(id: u64) -> Option<SnapshotSummary<AccountId>> {
			self.inner.snapshots.read().get(&id).cloned()
		}

		/// Returns all main account and corresponding proxies at this point in time
		fn get_all_accounts_and_proxies() -> Vec<(AccountId, Vec<AccountId>)> {
			self.inner.get_all_accounts_and_proxies()
		}

		/// Returns snapshot generation intervals
		fn get_snapshot_generation_intervals() -> (u64, BlockNumber) {
			self.inner.get_snapshot_generation_intervals()
		}

		/// Gets pending snapshot if any
		fn pending_snapshot() -> Option<u64>{
			self.inner.pending_snapshot()
		}

		/// Returns Public Key of Whitelisted Orderbook Operator
		fn get_orderbook_opearator_key() -> Option<sp_core::ecdsa::Public>{
			self.inner.get_orderbook_opearator_key()
		}


		/// Returns last processed stid from last snapshot
		fn get_last_accepted_worker_nonce() -> u64{
			self.inner.get_last_accepted_worker_nonce()
		}

		/// Reads the current trading pair configs
		fn read_trading_pair_configs() -> Vec<(TradingPair, TradingPairConfig)>{
			self.inner.read_trading_pair_configs()
		}
		/// Returns the allowlisted asset ids
		fn get_allowlisted_assets() -> Vec<AssetId> {
			self.inner.get_allowlisted_assets()
		}
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

/// Helper function to convert keyring types to AuthorityId
pub(crate) fn make_ob_ids(keys: &[AccountKeyring]) -> Vec<AuthorityId> {
	keys.iter()
		.map(|key| {
			let seed = key.to_seed();
			orderbook_primitives::crypto::Pair::from_string(&seed, None)
				.unwrap()
				.public()
				.into()
		})
		.collect()
}

#[derive(Default)]
pub struct PeerData {
	is_validator: bool,
	peer_rpc_link: Option<UnboundedSender<ObMessage>>,
	working_state_root: Arc<RwLock<[u8; 32]>>,
	memory_db: Arc<RwLock<MemoryDB<RefHasher, HashKey<RefHasher>, Vec<u8>>>>,
	last_successful_block_number_snapshot_created: Arc<RwLock<BlockNumber>>,
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
		(
			client.as_block_import(),
			None,
			PeerData {
				is_validator: false,
				peer_rpc_link: None,
				working_state_root: Arc::new(Default::default()),
				memory_db: Arc::new(Default::default()),
				last_successful_block_number_snapshot_created: Arc::new(Default::default()),
			},
		)
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
				notifications_protocols: vec!["/ob/1".into()],
				is_authority: false,
				..Default::default()
			});
		}
		net
	}

	pub(crate) fn add_authority_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec!["/ob/1".into()],
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
	API::Api: ObApi<Block>,
{
	let workers = FuturesUnordered::new();
	for (peer_id, key, api, is_validator) in peers.into_iter() {
		let (sender, receiver) = futures::channel::mpsc::unbounded();
		net.peers[peer_id].data.peer_rpc_link = Some(sender);
		net.peers[peer_id].data.is_validator = is_validator;
		net.peers[peer_id].data.last_successful_block_number_snapshot_created =
			Arc::new(RwLock::new(0_u32.saturated_into()));
		net.peers[peer_id].data.memory_db = Arc::new(RwLock::new(MemoryDB::default()));
		net.peers[peer_id].data.working_state_root = Arc::new(RwLock::new([0; 32]));

		let mut keystore = None;

		if is_validator {
			// Generate the crypto material with test keys,
			// we have to use file based keystore,
			// in memory keystore doesn't seem to work here
			keystore = Some(Arc::new(
				LocalKeystore::open(format!("keystore-{:?}", peer_id), None).unwrap(),
			));
			let (pair, _seed) =
				orderbook_primitives::crypto::Pair::from_string_with_seed(&key.to_seed(), None)
					.unwrap();
			// Insert the key
			keystore
				.as_mut()
				.unwrap()
				.insert_unknown(
					orderbook_primitives::KEY_TYPE,
					&key.to_seed(),
					pair.public().as_ref(),
				)
				.await
				.unwrap();
			// Check if the key is present or not
			keystore
				.as_ref()
				.unwrap()
				.key_pair::<orderbook_primitives::crypto::Pair>(&pair.public())
				.unwrap();
		}

		let ob_params = crate::ObParams {
			client: net.peers[peer_id].client().as_client(),
			backend: net.peers[peer_id].client().as_backend(),
			runtime: api,
			keystore,
			network: net.peers[peer_id].network_service().clone(),
			prometheus_registry: None,
			protocol_name: "/ob/1".into(),
			is_validator,
			message_sender_link: receiver,
			marker: Default::default(),
			memory_db: net.peers[peer_id].data.memory_db.clone(),
			working_state_root: net.peers[peer_id].data.working_state_root.clone(),
		};
		let gadget = crate::start_orderbook_gadget::<_, _, _, _, _>(ob_params);

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
