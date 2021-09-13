// Copyright (C) 2020-2021 Parity Technologies (UK) Ltd.
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

#![allow(clippy::collapsible_match)]

use std::{collections::BTreeSet, fmt::Debug, marker::PhantomData, sync::Arc};
use std::collections::HashMap;
use std::convert::TryFrom;

use codec::{Codec, Decode, Encode};
use futures::{future, FutureExt, StreamExt};
use ipfs_embed::{Config, DefaultParams, generate_keypair, Ipfs, Multiaddr, PeerId, Cid};
use log::{debug, error, info, trace, warn};
use parking_lot::Mutex;
use sc_client_api::{Backend, FinalityNotification, FinalityNotifications};
use sp_api::BlockId;
use sp_arithmetic::traits::AtLeast32Bit;
use sp_runtime::{
    generic::OpaqueDigestItemId,
    SaturatedConversion,
    traits::{Block, Header, NumberFor},
};

use offchain_ipfs_primitives::IpfsApi;

use crate::Client;

pub(crate) struct WorkerParams<B, BE, C>
    where
        B: Block,
{
    pub client: Arc<C>,
    pub backend: Arc<BE>,
    pub block: PhantomData<B>,
}

pub(crate) struct IPFSWorker<B, C, BE, AccountId>
    where
        B: Block,
        BE: Backend<B>,
        C: Client<B, BE>,
        AccountId: Codec,
{
    client: Arc<C>,
    backend: Arc<BE>,
    finality_notifications: FinalityNotifications<B>,
    // Embedded IPFS Client
    // TODO: Check if params need to be optimized
    ipfs_client: Ipfs<DefaultParams>,
    // Known mutliaddrs from runtime
    known_peers: HashMap<AccountId, Vec<Multiaddr>>,
    // Latest CID given by enclave
    latest_cid: Option<Cid>,
    // CID approved by other validators
    approved_cid: Option<Cid>,
    // keep rustc happy
    _backend: PhantomData<BE>,
    _account: PhantomData<AccountId>,
}

impl<B, C, BE, AccountId> IPFSWorker<B, C, BE, AccountId>
    where
        B: Block,
        BE: Backend<B>,
        C: Client<B, BE>,
        C::Api: IpfsApi<B, AccountId>,
        AccountId: Codec,
{
    pub(crate) async fn new(worker_params: WorkerParams<B, BE, C>) -> Self {
        let WorkerParams { client, backend, block } = worker_params;

        // Create IPFS Client
        // TODO: Use substrate's keystore and even try to use AuthorityID specific to Offchain IPFS worker
        // TODO: Make the IPFS path configurable via cli
        let ipfs_client = Ipfs::<DefaultParams>::new(
            Config::new("./IPFS".as_ref(), generate_keypair())).await.unwrap();
        trace!(target: "offchain-ipfs", "IPFS Client started, PeerID: {}",ipfs_client.local_peer_id());

        ipfs_client.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();


        IPFSWorker {
            client: client.clone(),
            backend,
            finality_notifications: client.finality_notification_stream(),
            ipfs_client,
            known_peers: HashMap::new(),
            latest_cid: None,
            approved_cid: None,
            _backend: PhantomData,
            _account: PhantomData,
        }
    }
}

impl<B, C, BE, AccountId> IPFSWorker<B, C, BE, AccountId>
    where
        B: Block,
        BE: Backend<B>,
        C: Client<B, BE>,
        C::Api: IpfsApi<B, AccountId>,
        AccountId: Codec + Eq + std::hash::Hash,
{
    fn handle_finality_notification(&mut self, notification: FinalityNotification<B>) {
        trace!(target: "offchain-ipfs", "🥩 Finality notification: {:?}", notification);
        let at = BlockId::hash(notification.hash);
        if let Ok(emergency_flag) = self.client.runtime_api().check_emergency_closure(&at) {
            if !emergency_flag {
                // Exchange is running, don't process user claims
                if let Ok(enclave_multi_addrs) = self.client.runtime_api().collect_enclave_multiaddrs(&at) {
                    for (enclave, multiaddr_strings) in enclave_multi_addrs {
                        if let Some(stored_addrs) = self.known_peers.get_mut(&enclave) {
                            let mut new_addrs = vec![];
                            for addr_string in multiaddr_strings {
                                if let Ok(multiaddr) = Multiaddr::try_from(addr_string) {
                                    if !stored_addrs.contains(&multiaddr) {
                                        self.ipfs_client.add_external_address(multiaddr.clone());
                                        new_addrs.push(multiaddr);
                                    }
                                }
                            }
                            stored_addrs.append(&mut new_addrs);
                        } else {
                            // We don't have any stored multiaddrs for this enclave
                            let mut new_addrs = vec![];
                            for addr_string in multiaddr_strings {
                                if let Ok(multiaddr) = Multiaddr::try_from(addr_string) {
                                    self.ipfs_client.add_external_address(multiaddr.clone());
                                    new_addrs.push(multiaddr);
                                }
                            }
                            self.known_peers.insert(enclave, new_addrs);
                        }
                    }

                    // Check latest CID changes
                    if let Ok(Some(latest_cid)) = self.client.runtime_api().get_latest_cid(&at) {
                        if let Some(saved_latest_cid) = self.latest_cid.clone() {
                            if saved_latest_cid != latest_cid {
                                // TODO: Handle the recieved data, find a non-blocking way of syncing cids
                                self.ipfs_client.get(&latest_cid);
                                // TODO: verify the integrity of data recieved via IPFS
                                // TODO: Create an inherent approving the cid
                            }
                        }
                    }
                    // Check Approved CID changes
                    if let Ok(Some(approved_cid)) = self.client.runtime_api().get_approved_cid(&at) {
                        if let Some(saved_approved_cid) = self.approved_cid.clone() {
                            if saved_approved_cid != approved_cid {
                                // TODO: Handle the recieved data, find a non-blocking way of syncing cids
                                self.ipfs_client.get(&approved_cid);
                                // TODO: Parse and Store the balance state to disc
                            }
                        }
                    }
                }
            } else {
                // Exchange is down, process only user claims
            }
        }
    }

    pub(crate) async fn run(mut self) {
        loop {
            futures::select! {
				notification = self.finality_notifications.next().fuse() => {
					if let Some(notification) = notification {
						self.handle_finality_notification(notification);
					} else {
						return;
					}
				},
			}
        }
    }
}
