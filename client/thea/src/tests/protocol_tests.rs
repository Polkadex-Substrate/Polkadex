// This file is part of Polkadex.

// Copyright (C) 2020-2022 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Thea client testing 

use crate::{
	keystore::tests::{Keyring as TheaKeyring, Keyring},
	start_thea_gadget, TheaParams,
};
use codec::{Decode, Encode};
use futures::{future, stream::FuturesUnordered, Future, FutureExt, StreamExt};
use parking_lot::{Mutex, RwLock};
use sc_consensus::BoxJustificationImport;
use sc_finality_grandpa::{
	run_grandpa_voter, Config, GenesisAuthoritySetProvider, GrandpaParams, LinkHalf,
	SharedVoterState,
};
use sc_keystore::LocalKeystore;
use sc_network::config::{ProtocolConfig, Role};
use sc_network_test::{
	Block, BlockImportAdapter, FullPeerConfig, Hash, PassThroughVerifier, Peer, PeersClient,
	PeersFullClient, TestNetFactory,
};
use serde::{Deserialize, Serialize};
use sp_api::{ApiRef, BlockId, ProvideRuntimeApi};
use sp_application_crypto::Pair as SPPair;
use sp_consensus::BlockOrigin;
use sp_core::{crypto::key_types::GRANDPA, sr25519::Pair};
use sp_finality_grandpa::{
	AuthorityList, EquivocationProof, GrandpaApi, OpaqueKeyOwnershipProof, SetId,
};
use sp_runtime::{traits::Header as HeaderT, BuildStorage, DigestItem};
use std::{
	pin::Pin,
	sync::{Arc, Mutex as StdMutex},
	task::Poll,
	thread::sleep,
	time::Duration,
};
use substrate_test_runtime_client::{
	runtime::Header, Ed25519Keyring, LongestChain, SyncCryptoStore, SyncCryptoStorePtr,
};
use thea_primitives::{
	constants::{MsgLimit, MsgVecLimit, PartialSignatureLimit, PartialSignatureVecLimit},
	crypto::Signature,
	keygen::{KeygenRound, OfflineStageRound, SigningSessionPayload, TheaPayload},
	payload::{SignedTheaPayload, UnsignedTheaPayload},
	AuthorityId, AuthorityIndex, ConsensusLog, PartyIndex, TheaApi, ValidatorSet,
	KEY_TYPE as TheaKeyType, THEA_ENGINE_ID,
};
use tokio::runtime::{Handle, Runtime};

type TestLinkHalf =
	LinkHalf<Block, PeersFullClient, LongestChain<substrate_test_runtime_client::Backend, Block>>;
type GrandpaPeerData = Mutex<Option<TestLinkHalf>>;
type GrandpaBlockImport = sc_finality_grandpa::GrandpaBlockImport<
	substrate_test_runtime_client::Backend,
	Block,
	PeersFullClient,
	LongestChain<substrate_test_runtime_client::Backend, Block>,
>;
type GrandpaPeer = Peer<GrandpaPeerData, GrandpaBlockImport>;

pub(crate) struct TheaTestNet {
	peers: Vec<GrandpaPeer>,
	test_net: Arc<TestApi>,
}

// same as runtime
pub(crate) type BlockNumber = u32;
pub(crate) type GrandpaBlockNumber = u64;

const THEA_PROTOCOL_NAME: &str = "THEA";
const GRANDPA_PROTOCOL_NAME: &str = "/paritytech/grandpa/1";
const TEST_GOSSIP_DURATION: Duration = Duration::from_millis(500);

impl TheaTestNet {
	pub(crate) fn new(n_authority: usize, n_full: usize, test_net: Arc<TestApi>) -> Self {
		let capacity = n_authority + n_full;
		let mut net = TheaTestNet { peers: Vec::with_capacity(capacity), test_net };
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
			notifications_protocols: vec![GRANDPA_PROTOCOL_NAME.into(), THEA_PROTOCOL_NAME.into()],
			is_authority: true,
			..Default::default()
		})
	}

	pub(crate) fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![GRANDPA_PROTOCOL_NAME.into(), THEA_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}
	pub(crate) fn generate_blocks(
		&mut self,
		count: usize,
		session_length: u64,
		validator_set: &ValidatorSet<AuthorityId>,
	) {
		self.peer(0).generate_blocks(count, BlockOrigin::File, |builder| {
			let mut block = builder.build().unwrap().block;

			if *block.header.number() % session_length == 0 {
				add_auth_change_digest(&mut block.header, validator_set.clone());
			}

			block
		});
	}
}

impl TestNetFactory for TheaTestNet {
	type Verifier = PassThroughVerifier;
	type BlockImport = GrandpaBlockImport;
	type PeerData = GrandpaPeerData;

	fn from_config(_config: &ProtocolConfig) -> Self {
		TheaTestNet { peers: Vec::new(), test_net: Default::default() }
	}

	fn make_verifier(
		&self,
		_client: PeersClient,
		_cfg: &ProtocolConfig,
		_: &GrandpaPeerData,
	) -> Self::Verifier {
		PassThroughVerifier::new(false) // use non-instant finality.
	}

	fn make_block_import(
		&self,
		client: PeersClient,
	) -> (
		BlockImportAdapter<Self::BlockImport>,
		Option<BoxJustificationImport<Block>>,
		GrandpaPeerData,
	) {
		let (client, backend) = (client.as_client(), client.as_backend());
		let (import, link) = sc_finality_grandpa::block_import(
			client.clone(),
			&*self.test_net,
			LongestChain::new(backend.clone()),
			None,
		)
		.expect("Could not create block import for fresh peer.");
		let justification_import = Box::new(import.clone());
		(BlockImportAdapter::new(import), Some(justification_import), Mutex::new(Some(link)))
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

	fn add_full_peer(&mut self) {
		self.add_full_peer_with_config(FullPeerConfig {
			notifications_protocols: vec![GRANDPA_PROTOCOL_NAME.into(), THEA_PROTOCOL_NAME.into()],
			is_authority: false,
			..Default::default()
		})
	}
}

fn add_auth_change_digest(header: &mut Header, new_auth_set: ValidatorSet<AuthorityId>) {
	header.digest_mut().push(DigestItem::Consensus(
		THEA_ENGINE_ID,
		ConsensusLog::<AuthorityId>::AuthoritiesChange(new_auth_set).encode(),
	));
}

#[derive(Serialize, Deserialize, Debug)]
struct Genesis(std::collections::BTreeMap<String, String>);

impl BuildStorage for Genesis {
	fn assimilate_storage(&self, storage: &mut sp_core::storage::Storage) -> Result<(), String> {
		storage
			.top
			.extend(self.0.iter().map(|(a, b)| (a.clone().into_bytes(), b.clone().into_bytes())));
		Ok(())
	}
}

pub(crate) fn make_thea_ids(keys: &[TheaKeyring]) -> Vec<AuthorityId> {
	keys.iter().map(|key| Pair::from(key.clone()).public().into()).collect()
}

pub(crate) fn create_thea_keystore(authority: TheaKeyring) -> SyncCryptoStorePtr {
	let keystore = Arc::new(LocalKeystore::in_memory());
	SyncCryptoStore::sr25519_generate_new(&*keystore, TheaKeyType, Some(&authority.to_seed()))
		.expect("Creates authority key");
	keystore
}

#[derive(Clone, Default)]
pub(crate) struct TestApi {
	genesys_validator_set: Vec<TheaKeyring>,
	next_validator_set: Vec<TheaKeyring>,
	keygen_messages:
		Arc<StdMutex<Vec<TheaPayload<AuthorityId, KeygenRound, MsgLimit, MsgVecLimit>>>>,
	signed_payloads: Arc<StdMutex<Vec<SignedTheaPayload>>>,
	signing_payloads: Arc<
		StdMutex<
			Vec<
				SigningSessionPayload<AuthorityId, PartialSignatureLimit, PartialSignatureVecLimit>,
			>,
		>,
	>,
	unsigned_payloads: Arc<StdMutex<Vec<UnsignedTheaPayload>>>,
	offline_messages:
		Arc<StdMutex<Vec<TheaPayload<AuthorityId, OfflineStageRound, MsgLimit, MsgVecLimit>>>>,
	genesis_authorities: AuthorityList,
	last_keygen_round: Arc<StdMutex<KeygenRound>>,
	validator_set_changed: Arc<StdMutex<bool>>,
}

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
	impl GrandpaApi<Block> for RuntimeApi {
		fn grandpa_authorities(&self) -> AuthorityList {
			self.inner.genesis_authorities.clone()
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

	impl TheaApi<Block> for RuntimeApi {
		fn validator_set(&self) -> thea_primitives::ValidatorSet<thea_primitives::AuthorityId> {
			ValidatorSet::new(make_thea_ids(&self.inner.genesys_validator_set), 0)
		}

		fn next_validator_set(&self) -> thea_primitives::ValidatorSet<thea_primitives::AuthorityId> {
			ValidatorSet::new(make_thea_ids(&self.inner.next_validator_set), 1)
		}

		fn submit_keygen_message(&self, payload: TheaPayload<AuthorityId, KeygenRound, MsgLimit, MsgVecLimit>,
			_signature: thea_primitives::AuthoritySignature, _rng: u64) -> Result<(), thea_primitives::SigningError>{
			*self.inner.last_keygen_round.lock().unwrap() = payload.round;
			self.inner.keygen_messages.lock().unwrap().push(payload);
			Ok(())
		}

		fn submit_offline_message(payload: TheaPayload<AuthorityId, OfflineStageRound, MsgLimit, MsgVecLimit>,
			_signature: thea_primitives::AuthoritySignature, _rng: u64, _payload_array: &[u8; 32]) -> Result<(), thea_primitives::SigningError>{
			self.inner.offline_messages.lock().unwrap().push(payload);
			Ok(())
		}

		fn submit_signing_message(_at: BlockNumber, payload: SigningSessionPayload<thea_primitives::AuthorityId, PartialSignatureLimit, PartialSignatureVecLimit>,
			_signature: thea_primitives::AuthoritySignature, _rng: u64) -> Result<(), thea_primitives::SigningError>{
			Ok(self.inner.signing_payloads.lock().unwrap().push(payload))
		}

		fn submit_signed_payload(&self, payload: SignedTheaPayload, _rng: u64) -> Result<(), thea_primitives::SigningError>{
			Ok(self.inner.signed_payloads.lock().unwrap().push(payload))
		}

		fn keygen_messages_api(party_idx: thea_primitives::PartyIndex, round: thea_primitives::keygen::KeygenRound) -> TheaPayload<thea_primitives::AuthorityId, thea_primitives::keygen::KeygenRound, MsgLimit, MsgVecLimit>{
			match self.inner.keygen_messages.lock().unwrap().iter().find(|m| m.auth_idx == party_idx && m.round == round) {
				Some(v) => v.clone(),
				None => TheaPayload { round: KeygenRound::Unknown, ..Default::default() }
			}
		}

		fn offline_messages_api(party_idx: PartyIndex, round: OfflineStageRound, _payload: &[u8; 32]) -> TheaPayload<AuthorityId, OfflineStageRound, MsgLimit, MsgVecLimit>{
			match self.inner.offline_messages.lock().unwrap().iter().find(|m| m.auth_idx == party_idx && m.round == round) {
				Some(v) => v.clone(),
				None => TheaPayload {round: OfflineStageRound::Unknown, ..Default::default()}
			}
		}

		fn signing_messages_api(_at: BlockNumber) -> Vec<SigningSessionPayload<AuthorityId, PartialSignatureLimit, PartialSignatureVecLimit>>{
			self.inner.signing_payloads.lock().unwrap().clone()
		}

		fn unsigned_payloads_api(_at: BlockNumber) -> Vec<UnsignedTheaPayload>{
			self.inner.unsigned_payloads.lock().unwrap().clone()
		}
		fn signed_payloads_api(_at: BlockNumber) -> Vec<SignedTheaPayload>{
			self.inner.signed_payloads.lock().unwrap().clone()
		}

		fn clean_keygen_messages(&self, _auth_idx: AuthorityIndex, _signature: thea_primitives::AuthoritySignature, _rng: u64) -> Result<(),thea_primitives::SigningError>{
			*self.inner.keygen_messages.lock().unwrap() = vec![];
			//*self.inner.signed_payloads.lock().unwrap() = vec![];
			//*self.inner.signing_payloads.lock().unwrap() = vec![];
			Ok(())
		}

		fn is_validator_set_changed(&self) -> bool {
			*self.inner.validator_set_changed.lock().unwrap()
		}
		fn register_offence(signature: thea_primitives::AuthoritySignature, offence: thea_primitives::keygen::OffenseReport<thea_primitives::AuthorityId>) ->  Result<(),thea_primitives::SigningError>{
			Ok(())
        }

	}
}

impl GenesisAuthoritySetProvider<Block> for TestApi {
	fn get(&self) -> sp_blockchain::Result<AuthorityList> {
		Ok(self.genesis_authorities.clone())
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

// Spawns grandpa voters. Returns a future to spawn on the runtime.
fn initialize_grandpa(
	net: &mut TheaTestNet,
	grandpa_peers: &[Ed25519Keyring],
) -> impl Future<Output = ()> {
	let voters = FuturesUnordered::new();

	// initializing grandpa gadget per peer
	for (peer_id, key) in grandpa_peers.iter().enumerate() {
		let (keystore, _) = create_keystore(*key);

		let (net_service, link) = {
			// temporary needed for some reason
			let link =
				net.peers[peer_id].data.lock().take().expect("link initialized at startup; qed");
			(net.peers[peer_id].network_service().clone(), link)
		};

		let grandpa_params = GrandpaParams {
			config: Config {
				gossip_duration: TEST_GOSSIP_DURATION,
				justification_period: 32,
				keystore: Some(keystore),
				name: Some(format!("peer#{}", peer_id)),
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

// Spawns thea workers. Returns a future to spawn on the runtime.
fn initialize_thea<API>(
	net: &mut TheaTestNet,
	peers: Vec<(usize, &TheaKeyring, Arc<API>)>,
) -> impl Future<Output = ()>
where
	API: ProvideRuntimeApi<Block> + Send + Sync + Default,
	API::Api: TheaApi<Block>,
{
	let thea_workers = FuturesUnordered::new();

	let (sender, _) = std::sync::mpsc::channel();
	let rpc_send = Arc::new(std::sync::Mutex::new(sender));

	// initializing thea gadget per peer
	for (peer_id, key, api) in peers.into_iter() {
		let peer = &net.peers[peer_id];

		let keystore = create_thea_keystore(*key);

		let rpc_send = rpc_send.clone();
		let thea_params = TheaParams {
			client: peer.client().as_client(),
			backend: peer.client().as_backend(),
			runtime: api.clone(),
			key_store: Some(keystore),
			rpc_send,
		};
		let gadget = start_thea_gadget::<_, _, _, _>(thea_params);

		fn assert_send<T: Send>(_: &T) {}
		assert_send(&gadget);
		thea_workers.push(gadget);
	}

	thea_workers.for_each(|_| async move {})
}

fn block_until_complete(
	future: impl Future + Unpin,
	net: &Arc<Mutex<TheaTestNet>>,
	runtime: &mut Runtime,
) {
	let drive_to_completion = futures::future::poll_fn(|cx| {
		net.lock().poll(cx);
		Poll::<()>::Pending
	});
	runtime.block_on(future::select(future, drive_to_completion));
}

// run the voters to completion. provide a closure to be invoked after
// the voters are spawned but before blocking on them.
fn run_to_completion_with<F>(
	runtime: &mut Runtime,
	blocks: u64,
	net: Arc<Mutex<TheaTestNet>>,
	peers: &[TheaKeyring],
	with: F,
) -> u64
where
	F: FnOnce(Handle) -> Option<Pin<Box<dyn Future<Output = ()>>>>,
{
	let mut wait_for = Vec::new();

	let highest_finalized = Arc::new(RwLock::new(0));

	if let Some(f) = (with)(runtime.handle().clone()) {
		wait_for.push(f);
	};

	for (peer_id, _) in peers.iter().enumerate() {
		let highest_finalized = highest_finalized.clone();
		let client = net.lock().peers[peer_id].client().clone();

		wait_for.push(Box::pin(
			client
				.finality_notification_stream()
				.take_while(move |n| {
					let mut highest_finalized = highest_finalized.write();
					if *n.header.number() > *highest_finalized {
						*highest_finalized = *n.header.number();
					}
					future::ready(n.header.number() < &blocks)
				})
				.collect::<Vec<_>>()
				.map(|_| ()),
		));
	}

	// wait for all finalized on each.
	let wait_for = ::futures::future::join_all(wait_for);

	block_until_complete(wait_for, &net, runtime);
	let highest_finalized = *highest_finalized.read();
	highest_finalized
}

fn run_to_completion(
	runtime: &mut Runtime,
	blocks: u64,
	net: Arc<Mutex<TheaTestNet>>,
	peers: &[TheaKeyring],
) -> u64 {
	run_to_completion_with(runtime, blocks, net, peers, |_| None)
}

fn make_gradpa_ids(keys: &[Ed25519Keyring]) -> AuthorityList {
	keys.iter().map(|key| key.clone().public().into()).map(|id| (id, 1)).collect()
}

fn full_keygen_cycle(
	net: Arc<Mutex<TheaTestNet>>,
	thea_api: Arc<TestApi>,
	runtime: &mut Runtime,
	validator_set: ValidatorSet<AuthorityId>,
	peers: &[Keyring],
	start_block: u64,
) {
	let sleep_time_sec = Duration::from_secs(5);
	// first block
	net.lock().generate_blocks(1, 10, &validator_set);
	net.lock().block_until_sync();
	// Verify all peers synchronized
	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().best_number,
			start_block,
			"Peer #{} failed to sync",
			i
		);
	}

	run_to_completion(runtime, start_block, net.clone(), peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			start_block,
			"Peer #{} failed to finalize",
			i
		);
	}
	sleep(sleep_time_sec);
	assert_ne!(*thea_api.last_keygen_round.lock().unwrap(), KeygenRound::Unknown);

	// second block
	net.lock().generate_blocks(1, 10, &validator_set);
	net.lock().block_until_sync();
	let mut next_block = start_block + 1;
	// Verify all peers synchronized
	for i in 0..3 {
		// checking if all three validator submitted first round payloads
		assert_ne!(
			thea_api
				.runtime_api()
				.keygen_messages_api(
					&BlockId::Number(next_block),
					i as u16 as PartyIndex,
					KeygenRound::Round1
				)
				.unwrap(),
			TheaPayload { round: KeygenRound::Unknown, ..Default::default() }
		);
		assert_eq!(
			net.lock().peer(i).client().info().best_number,
			next_block,
			"Peer #{} failed to sync",
			i
		);
	}

	run_to_completion(runtime, next_block, net.clone(), peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			next_block,
			"Peer #{} failed to finalize",
			i
		);
	}
	sleep(sleep_time_sec);

	// third block
	net.lock().generate_blocks(1, 10, &validator_set);
	net.lock().block_until_sync();
	next_block += 1;
	// Verify all peers synchronized
	for i in 0..3 {
		// checking if all three validator submitted second round payloads
		assert_ne!(
			thea_api
				.runtime_api()
				.keygen_messages_api(
					&BlockId::Number(next_block),
					i as u16 as PartyIndex,
					KeygenRound::Round2
				)
				.unwrap(),
			TheaPayload { round: KeygenRound::Unknown, ..Default::default() }
		);
		assert_eq!(
			net.lock().peer(i).client().info().best_number,
			next_block,
			"Peer #{} failed to sync",
			i
		);
	}

	run_to_completion(runtime, next_block, net.clone(), peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			next_block,
			"Peer #{} failed to finalize",
			i
		);
	}
	sleep(sleep_time_sec);

	// fourth block
	net.lock().generate_blocks(1, 10, &validator_set);
	net.lock().block_until_sync();
	next_block += 1;
	// Verify all peers synchronized
	for i in 0..3 {
		// checking if all three validator submitted second round payloads
		assert_ne!(
			thea_api
				.runtime_api()
				.keygen_messages_api(
					&BlockId::Number(next_block),
					i as u16 as PartyIndex,
					KeygenRound::Round3
				)
				.unwrap(),
			TheaPayload { round: KeygenRound::Unknown, ..Default::default() }
		);
		assert_eq!(
			net.lock().peer(i).client().info().best_number,
			next_block,
			"Peer #{} failed to sync",
			i
		);
	}

	run_to_completion(runtime, next_block, net.clone(), peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			next_block,
			"Peer #{} failed to finalize",
			i
		);
	}
	sleep(sleep_time_sec);

	// fifth block - keygen should be completed by now
	net.lock().generate_blocks(1, 10, &validator_set);
	net.lock().block_until_sync();
	next_block += 1;

	// Verify all peers synchronized
	for i in 0..3 {
		// checking if all three validator submitted second round payloads
		assert_ne!(
			thea_api
				.runtime_api()
				.keygen_messages_api(
					&BlockId::Number(next_block),
					i as u16 as PartyIndex,
					KeygenRound::Round4
				)
				.unwrap(),
			TheaPayload { round: KeygenRound::Unknown, ..Default::default() }
		);
		assert_eq!(
			net.lock().peer(i).client().info().best_number,
			next_block,
			"Peer #{} failed to sync",
			i
		);
	}

	run_to_completion(runtime, next_block, net.clone(), peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			next_block,
			"Peer #{} failed to finalize",
			i
		);
	}
	sleep(sleep_time_sec);

	// few more blocks to see all is good and to finish the session
	net.lock().generate_blocks(5, 10, &validator_set);
	net.lock().block_until_sync();
	// cleaning up our keygen payloads
	thea_api
		.runtime_api()
		.clean_keygen_messages(
			&BlockId::Number(next_block),
			0,
			Signature::decode(&mut [0u8; 64].as_ref()).unwrap(),
			0,
		)
		.unwrap();
	// let it soack in
	sleep(sleep_time_sec);
}

#[test]
fn thea_keygen_completes() {
	// TODO: uncomment this after CI can filter out Grandpa stopped errors
	// Uncomment to get sp_tracing errors output
	//sp_tracing::try_init_simple();

	// our runtime for the test chain
	let mut runtime = Runtime::new().unwrap();

	// creating 3 validators
	let peers = &[TheaKeyring::Alice, TheaKeyring::Bob, TheaKeyring::Charlie];
	let grandpa_peers = &[Ed25519Keyring::Alice, Ed25519Keyring::Bob, Ed25519Keyring::Charlie];
	let voters = make_gradpa_ids(grandpa_peers);

	// setting initial thea id to 0
	let validator_set = ValidatorSet::new(make_thea_ids(peers), 0);
	let thea_api = Arc::new(TestApi {
		genesys_validator_set: vec![TheaKeyring::Alice, TheaKeyring::Bob, TheaKeyring::Charlie],
		next_validator_set: vec![TheaKeyring::Alice, TheaKeyring::Charlie, TheaKeyring::Dave],
		genesis_authorities: voters,
		..Default::default()
	});

	// our thea network with 3 authorities and 1 full peer
	let mut network = TheaTestNet::new(3, 1, thea_api.clone());
	let thea_peers = peers
		.iter()
		.enumerate()
		.map(|(id, p)| (id, p, thea_api.clone()))
		.collect::<Vec<_>>();

	runtime.spawn(initialize_grandpa(&mut network, grandpa_peers));
	runtime.spawn(initialize_thea(&mut network, thea_peers));

	// Pushing 20 block - thea keygen should be done after this point
	network.generate_blocks(20, 10, &validator_set);
	network.block_until_sync();

	// Verify all peers synchronized
	for i in 0..3 {
		assert_eq!(network.peer(i).client().info().best_number, 20, "Peer #{} failed to sync", i);
	}

	let net = Arc::new(Mutex::new(network));

	run_to_completion(&mut runtime, 20, net.clone(), peers);

	for i in 0..3 {
		assert_eq!(
			net.lock().peer(i).client().info().finalized_number,
			20,
			"Peer #{} failed to finalize",
			i
		);
	}

	// we need this as otherwise we'll end up checking storage before actual work is done in async
	// tasks
	sleep(Duration::from_secs(300));
}

#[test]
fn thea_keygen_block_by_block() {
	// TODO: uncomment this after CI can filter out Grandpa stopped errors
	// Uncomment to get sp_tracing errors output
	//sp_tracing::try_init_simple();

	// our runtime for the test chain
	let mut runtime = Runtime::new().unwrap();

	// creating 3 validators
	let peers = &[TheaKeyring::Alice, TheaKeyring::Bob, TheaKeyring::Charlie, TheaKeyring::Dave];
	let grandpa_peers = &[
		Ed25519Keyring::Alice,
		Ed25519Keyring::Bob,
		Ed25519Keyring::Charlie,
		Ed25519Keyring::Dave,
	];
	let voters = make_gradpa_ids(grandpa_peers);

	// setting initial thea id to 0
	let validator_set = ValidatorSet::new(make_thea_ids(peers), 0);
	let thea_api = Arc::new(TestApi {
		genesys_validator_set: vec![TheaKeyring::Alice, TheaKeyring::Bob, TheaKeyring::Charlie],
		next_validator_set: vec![TheaKeyring::Alice, TheaKeyring::Charlie, TheaKeyring::Dave],
		genesis_authorities: voters,
		..Default::default()
	});

	// our thea network with 3 authorities and 1 full peer
	let mut network = TheaTestNet::new(4, 0, thea_api.clone());
	let thea_peers = peers
		.iter()
		.enumerate()
		.map(|(id, p)| (id, p, thea_api.clone()))
		.collect::<Vec<_>>();

	runtime.spawn(initialize_grandpa(&mut network, grandpa_peers));
	runtime.spawn(initialize_thea(&mut network, thea_peers));

	let net = Arc::new(Mutex::new(network));

	// run first keygen block by block
	// we start with block 1
	full_keygen_cycle(
		net.clone(),
		thea_api.clone(),
		&mut runtime,
		validator_set.clone(),
		peers,
		1u64,
	);

	sleep(Duration::from_secs(20));

	// confirming we went through all rounds
	assert_eq!(*thea_api.last_keygen_round.lock().unwrap(), KeygenRound::Round4);

	// Now rotating validators
	*thea_api.validator_set_changed.lock().unwrap() = true;

	// now we've set validator set changed and start from block 11
	full_keygen_cycle(net.clone(), thea_api.clone(), &mut runtime, validator_set, peers, 11u64);

	sleep(Duration::from_secs(20));
	// Here if no errors? we are good with validator change!
}
