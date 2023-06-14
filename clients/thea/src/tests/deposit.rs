// This file is part of Polkadex.
//
// Copyright (c) 2023 Polkadex oü.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use crate::{
	connector::traits::ForeignConnector,
	error::Error,
	tests::{
		create_workers_array, generate_and_finalize_blocks, make_thea_ids, TestApi, TheaTestnet,
	},
	types::GossipMessage,
};
use async_trait::async_trait;
use futures::StreamExt;
use log::info;
use parking_lot::RwLock;
use sp_keyring::AccountKeyring;
use std::{collections::BTreeMap, sync::Arc, time::Duration};

use thea_primitives::{AuthorityId, Message, ValidatorSet};

pub struct DummyForeignConnector {
	active: Vec<AuthorityId>,
}

#[async_trait]
impl ForeignConnector for DummyForeignConnector {
	fn block_duration(&self) -> Duration {
		Duration::from_secs(12)
	}

	async fn connect(_url: String) -> Result<Self, Error>
	where
		Self: Sized,
	{
		Ok(DummyForeignConnector { active: vec![] })
	}

	async fn read_events(&self, last_processed_nonce: u64) -> Result<Option<Message>, Error> {
		if last_processed_nonce == 1 {
			Ok(Some(Message {
				block_no: 10,
				nonce: 1,
				data: vec![1, 2, 3],
				network: 1,
				is_key_change: false,
				validator_set_id: 0,
				validator_set_len: self.active.len() as u64,
			}))
		} else {
			// No new messages after nonce 1
			Ok(None)
		}
	}

	async fn send_transaction(&self, _message: GossipMessage) -> Result<(), Error> {
		unimplemented!()
	}

	async fn check_message(&self, message: &Message) -> Result<bool, Error> {
		info!(target:"thea-test", "Checking new message...");
		Ok(message ==
			&Message {
				block_no: 10,
				nonce: 1,
				data: vec![1, 2, 3],
				network: 1,
				is_key_change: false,
				validator_set_id: 0,
				validator_set_len: self.active.len() as u64,
			})
	}

	async fn last_processed_nonce_from_native(&self) -> Result<u64, Error> {
		Ok(0)
	}

	async fn check_thea_authority_initialization(&self) -> Result<bool, Error> {
		Ok(!self.active.is_empty())
	}
}

#[tokio::test]
#[ignore]
#[serial_test::serial]
pub async fn test_foreign_deposit() {
	sp_tracing::try_init_simple();

	let network = 1;
	let peers = &[
		(AccountKeyring::Alice, true),
		(AccountKeyring::Bob, true),
		(AccountKeyring::Charlie, true),
	];

	let message = Message {
		block_no: 10,
		nonce: 1,
		data: vec![1, 2, 3],
		network: 1,
		is_key_change: false,
		validator_set_id: 0,
		validator_set_len: 3,
	};
	let active: Vec<AuthorityId> =
		make_thea_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

	let runtime = Arc::new(TestApi {
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
		outgoing_messages: BTreeMap::new(),
		incoming_messages: Arc::new(RwLock::new(BTreeMap::new())),
		incoming_nonce: Arc::new(RwLock::new(BTreeMap::new())),
		_outgoing_nonce: BTreeMap::new(),
	});

	let mut testnet = TheaTestnet::new(3, 1, runtime.clone());
	let foreign_connector = Arc::new(DummyForeignConnector { active });

	let validators = peers
		.iter()
		.enumerate()
		.map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth, foreign_connector.clone()))
		.collect();

	let mut workers = create_workers_array(&mut testnet, validators).await;

	// We have created two thea validator worker nodes - let the fun begin!

	generate_and_finalize_blocks(1, &mut testnet).await;

	assert_eq!(workers.len(), 3);
	//  Check for new message from foreign chain
	let mut count = 1;
	for (worker, finality_future) in &mut workers {
		info!(target:"test", "Waiting for next finalized event; worker id: {count:?}");
		let next_finalized_blk = finality_future.next().await.unwrap();
		// Progress the worker's chain
		worker.handle_finality_notification(&next_finalized_blk).await.unwrap();
		// Progress the worker's foreign driver
		worker.try_process_foreign_chain_events().await.unwrap();
		assert_eq!(worker.message_cache.read().len(), 1);
		count += 1;
	}
	// At this point, all workers generated their own message, signed and gossiped it.
	// not if we artificially gossip these messages to each other.

	// Get all the messages
	let _message0 = workers[0].0.message_cache.read().get(&message).cloned().unwrap();
	let message1 = workers[1].0.message_cache.read().get(&message).cloned().unwrap();
	let message2 = workers[2].0.message_cache.read().get(&message).cloned().unwrap();

	// Send 1,2 to 0
	workers[0]
		.0
		.process_gossip_message(&mut message1.1.clone(), None)
		.await
		.unwrap(); // We got majority here
	assert_eq!(runtime.incoming_messages.read().len(), 1);
	assert_eq!(*runtime.incoming_nonce.read().get(&network).unwrap(), 1);
	// We can't assert_eq the full message as the signature is different due to aggregation
	assert_eq!(runtime.incoming_messages.read().get(&(network, 1)).unwrap().data, message.data);
	assert!(workers[0].0.message_cache.read().is_empty());
	workers[0]
		.0
		.process_gossip_message(&mut message2.1.clone(), None)
		.await
		.unwrap();
	assert!(workers[0].0.message_cache.read().is_empty());

	// Check for new events and should return no new messages on foreign
	workers[0].0.try_process_foreign_chain_events().await.unwrap();
	assert!(workers[0].0.message_cache.read().is_empty());
}
