use std::hash::Hash;
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};
use std::sync::{Arc, Mutex};
use codec::Decode;
use jsonrpsee::core::client::ClientT;
use std::ops::Deref;
use std::thread::current;
use sp_core::storage::StorageKey;
use async_trait::async_trait;
use polkadex_primitives::ingress::IngressMessages;
use jsonrpsee::rpc_params;
use polkadex_primitives::AccountId;
use serde_json::{Number, to_value};
use serde_json::Value;
use serde_json::Value::Null;
use sp_core::{H256, U256};
use rustc_hex::ToHex;
use substrate_api_client::FromHexString;
use std::str::FromStr;
use std::time::Duration;
use polkadex_primitives::Header;


const PALLET: &[u8] = b"OCEX";
// TODO: Replace with TheaMessages when it is Ready
const MESSAGES: &[u8] = b"IngressMessages";

#[async_trait]
pub trait NativeReader: Send + Sync {
    type Client;
    type BlockNo;
    type Messages;
    async fn get_messages(&self, block_no: Self::BlockNo) -> Option<Vec<Self::Messages>>;
}

pub trait NativeWriter {
    type Client;
    type KeyPair;
    fn send_transaction(thea_messages: Vec<u32>);
}

pub struct NativeConnector <R: NativeReader> {
    reader: R,
}

pub struct SubstrateReader {
    client: Arc<WsClient>,
}

// TODO: Wrapper Struct for Writer


#[async_trait]
impl NativeReader for SubstrateReader{
    type Client = WsClient;
    type BlockNo = sp_core::H256;
    type Messages = IngressMessages<AccountId>;

    async fn get_messages(&self, block_hash: Self::BlockNo) -> Option<Vec<Self::Messages>> {
        // Create Storage Key for Ingress Messages
        let mut bytes = sp_core::twox_128(PALLET).to_vec();
        bytes.extend(&sp_core::twox_128(MESSAGES)[..]);
        let storage_key: StorageKey = StorageKey(bytes);

        let client = self.client.clone();

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
    pub async fn new(node_url: &str) -> Self {
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

    pub async fn run(&self){
        let mut last_processed_block_hash: Option<H256> = None;
        let mut last_processed_block_number: u128 = 0;
        loop{
            let client = self.client.clone();
            // Fetch finalized blocks
            let block_hash: Value = client
                .request("chain_getFinalizedHead", rpc_params![])
                .await
                .unwrap();

            let block_hash = H256::from_str(block_hash.as_str().unwrap()).unwrap();

            // Fetch header
            let block_header: Value = client
                .request("chain_getHeader", rpc_params![block_hash])
                .await
                .unwrap();

            // Decode Block Header
            let block_header: Header = serde_json::from_str(&block_header.to_string()).unwrap();

            if let Some(hash) = last_processed_block_hash  {
                if block_hash == hash {
                    continue;
                }
                if block_header.parent_hash != hash{
                    panic!("Parents Mismatch");
                }
            } else {
                last_processed_block_hash = Some(block_hash);
            }

            // Fetch Ingress Messages
            let ingress_messages = self.get_messages(block_hash).await.unwrap();
            log::info!(target: "Polkadex Reader","Received Ingress Messages: {:?} for BlockHash: {:?}, Block Number: {:?}", ingress_messages, block_hash.clone(), block_header.number);

            // Update Last Processed Block Hash
            last_processed_block_hash = Some(block_hash);
            last_processed_block_number = block_header.number.into();

            // TODO: Submit block to Controller
        }
    }
}

impl <R: NativeReader>NativeConnector<R>{
    pub fn new(reader: R) -> Self {
        NativeConnector{
            reader
        }
    }
}



