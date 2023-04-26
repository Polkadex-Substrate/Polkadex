use super::*;

const TEST_GOSSIP_DURATION: Duration = Duration::from_millis(500);
pub(crate) const GRANDPA_PROTOCOL_NAME: &str = "/grandpa/1";
pub(crate) type GrandpaBlockNumber = u64;
pub(crate) type TestLinkHalf =
	LinkHalf<Block, PeersFullClient, LongestChain<substrate_test_runtime_client::Backend, Block>>;
pub(crate) type GrandpaPeerData = Mutex<Option<TestLinkHalf>>;
pub(crate) type GrandpaBlockImport = sc_finality_grandpa::GrandpaBlockImport<
	substrate_test_runtime_client::Backend,
	Block,
	PeersFullClient,
	LongestChain<substrate_test_runtime_client::Backend, Block>,
>;
pub(crate) type GrandpaPeer = Peer<GrandpaPeerData, GrandpaBlockImport>;

#[derive(Default)]
pub(crate) struct GrandpaTestnet {
	pub(crate) peers: Vec<GrandpaPeer>,
	api: Arc<TestApi>,
}

impl GrandpaTestnet {
	pub(crate) fn new(n_authority: usize, n_full: usize, api: Arc<TestApi>) -> Self {
		let mut net = GrandpaTestnet { peers: Vec::with_capacity(n_authority + n_full), api };
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
				crate::thea_protocol_name::NAME.into(),
			],
			is_authority: true,
			..Default::default()
		})
	}

	pub(crate) fn drop_validator(&mut self) {
		drop(self.peers.remove(0))
	}
}

impl TestNetFactory for GrandpaTestnet {
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
				crate::thea_protocol_name::NAME.into(),
			],
			is_authority: false,
			..Default::default()
		})
	}
}

fn create_keystore(authority: Ed25519Keyring) -> (SyncCryptoStorePtr, tempfile::TempDir) {
	let keystore_path = tempfile::tempdir().expect("Creates keystore path");
	let keystore =
		Arc::new(LocalKeystore::open(keystore_path.path(), None).expect("Creates keystore"));
	SyncCryptoStore::ed25519_generate_new(&*keystore, GRANDPA, Some(&authority.to_seed()))
		.expect("Creates authority key");

	(keystore, keystore_path)
}

pub(crate) fn initialize_grandpa(
	net: &mut TheaTestnet,
	peers: &[Ed25519Keyring],
) -> impl Future<Output = ()> {
	let voters = FuturesUnordered::new();

	// initializing grandpa gadget per peer
	for (peer_id, key) in peers.iter().enumerate() {
		let (keystore, _) = create_keystore(*key);

		let (net_service, link) = {
			// temporary needed for some reason
			let link = net.peers[peer_id]
				.data
				.lock()
				.unwrap()
				.take()
				.expect("link initialized at startup;config qed");
			(net.peers[peer_id].network_service().clone(), link)
		};

		let grandpa_params = GrandpaParams {
			config: Config {
				gossip_duration: TEST_GOSSIP_DURATION,
				justification_period: 32,
				keystore: Some(keystore),
				name: Some(format!("peer#{peer_id}")),
				local_role: Role::Authority,
				observer_enabled: true,
				telemetry: None,
				protocol_name: GRANDPA_PROTOCOL_NAME.into(),
			},
			link,
			network: net_service,
			voting_rule: (),
			prometheus_registry: None,
			shared_voter_state: SharedVoterState::empty(),
			telemetry: None,
		};
		let voter =
			run_grandpa_voter(grandpa_params).expect("all in order with client and network");

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&voter);
		voters.push(voter);
	}

	voters.for_each(|_| async move {})
}

pub(crate) fn make_gradpa_ids(keys: &[Ed25519Keyring]) -> AuthorityList {
	keys.iter().map(|key| (*key).public().into()).map(|id| (id, 1)).collect()
}
