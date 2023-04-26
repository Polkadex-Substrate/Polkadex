use std::{sync::Arc, time::Duration};

use futures::SinkExt;
use memory_db::MemoryDB;
use parking_lot::RwLock;
use primitive_types::H256;
use sc_network::Multiaddr;
use sc_network_common::service::{NetworkPeers, NetworkStateInfo};
use sc_network_test::{FullPeerConfig, TestNetFactory};
use sp_arithmetic::traits::SaturatedConversion;
use sp_consensus::BlockOrigin;
use sp_core::Pair;
use sp_keyring::AccountKeyring;

use orderbook_primitives::{
    crypto::AuthorityId,
    types::{ObMessage, UserActions},
};
use orderbook_primitives::types::GossipMessage;

use crate::tests::{
    generate_and_finalize_blocks, initialize_orderbook, make_ob_ids, ObTestnet, TestApi,
};

#[tokio::test]
pub async fn test_orderbook_gossip() {
    sp_tracing::try_init_simple();

    let (orderbook_operator, _) = sp_core::ecdsa::Pair::generate();
    let mut testnet = ObTestnet::new(3, 2);
    let peers = &[
        (AccountKeyring::Alice, true),
        (AccountKeyring::Bob, true),
        (AccountKeyring::Charlie, true),
    ];

    let active: Vec<AuthorityId> =
        make_ob_ids(&peers.iter().map(|(k, _)| k.clone()).collect::<Vec<AccountKeyring>>());

    let runtime = Arc::new(TestApi {
        active,
        latest_snapshot_nonce: Arc::new(Default::default()),
        snapshots: Arc::new(Default::default()),
        unprocessed: Arc::new(Default::default()),
        main_to_proxy_mapping: Default::default(),
        pending_snapshot: None,
        operator_key: Some(orderbook_operator.public()),
        trading_config: vec![],
        withdrawals: Arc::new(Default::default()),
        ingress_messages: vec![],
        allowlisted_assets: vec![],
    });

    let ob_peers = peers
        .iter()
        .enumerate()
        .map(|(id, (key, is_auth))| (id, key, runtime.clone(), *is_auth))
        .collect();

    let future = initialize_orderbook(&mut testnet, ob_peers).await;
    tokio::spawn(future);
    // Generate and finalize two block to start finality
    generate_and_finalize_blocks(1, &mut testnet, 3).await;
    generate_and_finalize_blocks(1, &mut testnet, 3).await;
    // Send the RPC with Ob message
    let mut message = ObMessage {
        worker_nonce: 1,
        stid: 10,
        action: UserActions::BlockImport(1),
        signature: Default::default(),
    };
    message.signature = orderbook_operator.sign_prehashed(&message.sign_data());
    testnet.peers[0]
        .data
        .peer_rpc_link
        .as_ref()
        .unwrap()
        .send(message)
        .await
        .unwrap();
    testnet.run_until_sync().await;

    // Generate and finalize one block
    generate_and_finalize_blocks(5, &mut testnet, 3).await;

    testnet.run_until_idle().await;

    /// We should have generated one snapshot by this point
    assert_eq!(runtime.snapshots.read().len(), 1);
    for peer in testnet.peers() {
        let state_root = H256::from_slice(&*peer.data.working_state_root.read());
        if peer.data.is_validator {
            assert_eq!(state_root, runtime.get_latest_snapshot().state_root);
        } else {
            println!(
                "Fullnode id: {:?}, root: {:?}",
                peer.network_service().local_peer_id(),
                state_root
            );
        }
    }

    // Add the new full node, this is the fifth node
    testnet.add_full_peer_with_config(FullPeerConfig {
        notifications_protocols: vec!["/ob/1".into()],
        is_authority: false,
        ..Default::default()
    });
    // Start the new node's worker
    let fifth_node_index = testnet.peers().len() - 1;

    let working_state_root = Arc::new(RwLock::new([0; 32]));

    let last_successful_block_number_snapshot_created =
        Arc::new(RwLock::new(0_u32.saturated_into()));

    let memory_db = Arc::new(RwLock::new(MemoryDB::default()));
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    // Now we add a new full node and see if it can catch up.
    let worker_params = crate::worker::WorkerParams {
        client: testnet.peers[fifth_node_index].client().as_client(),
        backend: testnet.peers[fifth_node_index].client().as_backend(),
        runtime: runtime.clone(),
        sync_oracle: testnet.peers[fifth_node_index].network_service().clone(),
        keystore: None,
        network: testnet.peers[fifth_node_index].network_service().clone(),
        protocol_name: "/ob/1".into(),
        is_validator: false,
        message_sender_link: receiver,
        _marker: Default::default(),
        last_successful_block_number_snapshot_created:
        last_successful_block_number_snapshot_created.clone(),
        memory_db: memory_db.clone(),
        working_state_root: working_state_root.clone(),
        metrics: None,
    };

    let mut worker = crate::worker::ObWorker::<_, _, _, _, _, _>::new(worker_params);
    let mut worker_nonce = 0;
    //check how gossip reacts to want message when it does not have it
    let mut gossips = worker.get_want_worker_nonce_messages(&1, &2);
    assert_eq!(gossips.len(), 0);

    //check how gossip reacts to want massages when it does have all of it
    for blk in 1..20 {
        worker_nonce += 1;
        let msg = create_ob_message_import_block(&orderbook_operator.seed(), worker_nonce, blk);
        worker.process_new_user_action(&msg);
    }
    gossips = worker.get_want_worker_nonce_messages(&1, &12);
    assert_eq!(gossips.len(), 1)
    //check how gossip reacts to have messages when it has some of it.
}

fn create_ob_message_import_block(seed: &[u8; 32], nonce: u64, blk_num: u32) -> ObMessage {
    // Send the RPC with Ob message
    let mut msg = ObMessage {
        worker_nonce: nonce,
        stid: nonce,
        action: UserActions::BlockImport(blk_num),
        signature: Default::default(),
    };
    let pair = sp_core::ecdsa::Pair::from_seed(seed);
    msg.signature = pair.sign_prehashed(&msg.sign_data());
    return msg;
}
