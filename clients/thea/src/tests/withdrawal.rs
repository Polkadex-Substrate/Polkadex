use crate::{
	connector::traits::ForeignConnector,
	error::Error,
	tests::{generate_and_finalize_blocks, initialize_thea, make_thea_ids, TestApi, TheaTestnet},
	types::GossipMessage,
};
use async_trait::async_trait;
use parking_lot::RwLock;
use sp_keyring::AccountKeyring;
use std::{collections::BTreeMap, sync::Arc, time::Duration};
use subxt::ext::frame_metadata::StorageEntryModifier::Default;
use thea_primitives::{AuthorityId, Message, ValidatorSet};

pub struct DummyForeignConnector;

#[async_trait]
impl ForeignConnector for DummyForeignConnector {
	fn block_duration(&self) -> Duration {
		Duration::from_secs(10)
	}

	async fn connect(url: String) -> Result<Self, Error>
	where
		Self: Sized,
	{
		Ok(DummyForeignConnector)
	}

	async fn read_events(&self, last_processed_nonce: u64) -> Result<Option<Message>, Error> {
		todo!()
	}

	async fn send_transaction(&self, message: GossipMessage) {
		todo!()
	}

	async fn check_message(&self, message: &Message) -> Result<bool, Error> {
		todo!()
	}

	async fn last_processed_nonce_from_native(&self) -> Result<u64, Error> {
		todo!()
	}
}

#[tokio::test]
pub async fn test_withdrawal() {
	sp_tracing::try_init_simple();

	let mut testnet = TheaTestnet::new(3, 0);
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let active: Vec<AuthorityId> =
		make_thea_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let runtime = Arc::new(TestApi {
		authorities: BTreeMap::from([(0, ValidatorSet { set_id: 0, validators: active.clone() })]),
		validator_set_id: 0,
		next_authorities: BTreeMap::new(),
		network_pref: BTreeMap::new(),
		outgoing_messages: BTreeMap::new(),
		incoming_messages: Arc::new(RwLock::new(BTreeMap::new())),
		incoming_nonce: Arc::new(RwLock::new(BTreeMap::new())),
		outgoing_nonce: BTreeMap::new(),
	});

	let foreign_connector = Arc::new(DummyForeignConnector);

	let ob_peers = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth, foreign_connector.clone()))
		.collect();

	let future = initialize_thea(&mut testnet, ob_peers).await;

	tokio::spawn(future);
	// Generate and finalize two block to start finality
	generate_and_finalize_blocks(1, &mut testnet, 3).await;
}
