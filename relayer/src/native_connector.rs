use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use std::sync::{Arc, Mutex};
use codec::Decode;
use hex::FromHex;
use jsonrpsee::core::client::ClientT;
use std::ops::Deref;
use sp_core::storage::StorageKey;
use async_trait::async_trait;
use polkadex_primitives::ingress::IngressMessages;
use jsonrpsee::rpc_params;
use polkadex_primitives::AccountId;
use serde_json::to_value;
use serde_json::Value;


const PALLET: &[u8] = b"OCEX";
// TODO: Replace with TheaMessages when it is Ready
const MESSAGES: &[u8] = b"IngressMessages";

#[async_trait]
pub trait NativeReader: Send + Sync {
    type Client;
    type BlockNo;
    type Messages;
    async fn get_messages(&mut self, block_no: Self::BlockNo) -> Option<Vec<Self::Messages>>;
}

pub trait NativeWriter {
    type Client;
    type KeyPair;
    fn send_transaction(thea_messages: Vec<u32>);
}

pub struct NativeConnector <R: NativeReader, W: NativeWriter> {
    reader: R,
    writer: W,
}

// TODO: Invoking object in Main Runtime thread to be done

pub struct SubstrateReader {
    client: Arc<WsClient>,
}

#[async_trait]
impl NativeReader for SubstrateReader{
    type Client = WsClient;
    type BlockNo = u32;
    type Messages = IngressMessages<AccountId>;

    async fn get_messages(&mut self, block_no: Self::BlockNo) -> Option<Vec<Self::Messages>> {
        // Create Storage Key for Ingress Messages
        let mut bytes = sp_core::twox_128(PALLET).to_vec();
        bytes.extend(&sp_core::twox_128(MESSAGES)[..]);
        let storage_key: StorageKey = StorageKey(bytes);

        let client = self.client.clone();

        // Fetch Block hash for the provided block number
        let block_hash: Value = client
            .request("chain_getBlockHash", rpc_params![block_no as u32])
            .await
            .unwrap();

        // JSON RPC request for Ingress Messages
        let ingress_messages: Value = client
            .request(
                "state_getStorage",
                rpc_params![to_value(storage_key.clone()).unwrap(), block_hash.clone()],
            )
            .await
            .unwrap();

        // Decode Ingress Messages if Not Null
        if ingress_messages != Value::Null {
            let mut bytes = Vec::from_hex(ingress_messages.to_string()).unwrap();
            let ingress_messages: Vec<IngressMessages<AccountId>> = Decode::decode(&mut bytes.as_slice()).unwrap();
            return Some(ingress_messages);
        }
        None
    }
}

impl SubstrateReader {
    async fn new(node_url: &str) -> Self {
        let client = WsClientBuilder::default()
            .max_request_body_size(u32::MAX)
            .max_concurrent_requests(1024 * 1024 * 1024)
            .request_timeout(std::time::Duration::from_secs(10000))
            .build(node_url)
            .await
            .unwrap();

        SubstrateReader{
            client: Arc::new(client)
        }
    }
}


