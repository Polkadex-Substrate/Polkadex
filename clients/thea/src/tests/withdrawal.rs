use crate::{
	connector::traits::ForeignConnector,
	error::Error,
	tests::{generate_and_finalize_blocks, initialize_thea, make_thea_ids, TestApi, TheaTestnet},
	types::GossipMessage,
};
use async_trait::async_trait;
use log::info;
use parking_lot::RwLock;
use sc_network_test::TestNetFactory;
use sp_keyring::AccountKeyring;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use subxt::ext::frame_metadata::StorageEntryModifier::Default;
use thea_primitives::{AuthorityId, Message, ValidatorSet};

pub struct DummyForeignConnector {
	active: Vec<AuthorityId>,
}

#[async_trait]
impl ForeignConnector for DummyForeignConnector {
	fn block_duration(&self) -> Duration {
		Duration::from_secs(1)
	}

	async fn connect(url: String) -> Result<Self, Error>
	where
		Self: Sized,
	{
		Ok(DummyForeignConnector { active: vec![] })
	}

	async fn read_events(&self, last_processed_nonce: u64) -> Result<Option<Message>, Error> {
		assert_eq!(last_processed_nonce, 1);
		Ok(Some(Message {
			block_no: 10,
			nonce: 1,
			data: vec![1, 2, 3],
			network: 0,
			is_key_change: false,
			validator_set_id: 0,
			validator_set_len: self.active.len() as u64,
		}))
	}

	async fn send_transaction(&self, message: GossipMessage) {
		todo!()
	}

	async fn check_message(&self, message: &Message) -> Result<bool, Error> {
		info!(target:"thea-test", "CHecking new message...");
		Ok(message ==
			&Message {
				block_no: 10,
				nonce: 1,
				data: vec![1, 2, 3],
				network: 0,
				is_key_change: false,
				validator_set_id: 0,
				validator_set_len: self.active.len() as u64,
			})
	}

	async fn last_processed_nonce_from_native(&self) -> Result<u64, Error> {
		Ok(0)
	}
}

#[tokio::test]
pub async fn test_withdrawal() {
	sp_tracing::try_init_simple();

	let mut testnet = TheaTestnet::new(3, 1);
	let network = 1;
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_thea_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let runtime = Arc::new(TestApi {
		authorities: BTreeMap::from([(
			network,
			ValidatorSet { set_id: 0, validators: active.clone() },
		)]),
		validator_set_id: 0,
		next_authorities: BTreeMap::new(),
		network_pref: BTreeMap::from([
			(active[0].clone(), network),
			(active[1].clone(), network),
			(active[2].clone(), network),
		]),
		outgoing_messages: BTreeMap::new(),
		incoming_messages: Arc::new(RwLock::new(BTreeMap::new())),
		incoming_nonce: Arc::new(RwLock::new(BTreeMap::new())),
		outgoing_nonce: BTreeMap::new(),
	});

	let foreign_connector = Arc::new(DummyForeignConnector { active });

	let ob_peers = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth, foreign_connector.clone()))
		.collect();

	let future = initialize_thea(&mut testnet, ob_peers).await;

	tokio::spawn(future);
	testnet.run_until_connected().await;
	// Generate and finalize two block to start finality
	generate_and_finalize_blocks(3, &mut testnet).await;
	testnet.run_until_sync().await;
	generate_and_finalize_blocks(3, &mut testnet).await;
	testnet.run_until_idle().await;

	tokio::time::sleep(Duration::from_secs(5)).await;

	assert_eq!(*runtime.incoming_nonce.read().get(&1).unwrap(), 1);
}
