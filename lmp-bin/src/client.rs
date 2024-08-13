use std::collections::BTreeMap;
use polkadex_primitives::rewards::Reward;
use sp_core::{Decode, Encode};
use subxt_signer::sr25519::{Keypair, Signature};
use subxt::config::SubstrateConfig;
use subxt::dynamic::Value;
use subxt::utils::{AccountId32, H256};
use subxt::{Config, OnlineClient, PolkadotConfig};
use subxt::config::polkadot::PolkadotExtrinsicParamsBuilder as Params;
use subxt_signer::bip39::Mnemonic;
use subxt_signer::sr25519::dev;
#[subxt::subxt(runtime_metadata_path = "src/metadata.scale")]
pub mod polkadex {}

#[derive(Clone, Debug)]
pub struct SubstrateClient {
    client: OnlineClient<SubstrateConfig>,
    signer: Keypair,
}

impl SubstrateClient {
    pub async fn initialize(url: String, phrase: String) -> Result<Self, String> {
        let api = OnlineClient::<SubstrateConfig>::from_url(url).await.unwrap();
        let mnemonic = Mnemonic::parse(phrase).unwrap();
        let signer = subxt_signer::sr25519::Keypair::from_phrase(&mnemonic, None).unwrap();
        let update_task = api.updater();
        tokio::spawn(async move {
            update_task
                .perform_runtime_updates()
                .await
                .expect("Expected the upgrade to work fine");
        });
        Ok(Self {
            client: api,
            signer,
        })
    }
        pub async fn submit_proposal(&self, proposal: BTreeMap<AccountId32, Reward>) -> Result<(), String> {
            let proposal = proposal.encode();
            let reward_map: BTreeMap<AccountId32, polkadex::runtime_types::polkadex_primitives::rewards::Reward> = Decode::decode(&mut &proposal[..]).unwrap();
            let rewrd_vec: Vec<(AccountId32, polkadex::runtime_types::polkadex_primitives::rewards::Reward)> = reward_map.into_iter().map(|(key,value)| (key, value)).collect();
            let reward_tx = polkadex::tx().ocex().submit_reward_proposal(rewrd_vec);
            let latest_block = self.client.blocks().at_latest().await.unwrap();
            let tx_params = Params::new()
                .tip(1_000)
                .mortal(latest_block.header(), 32)
                .build();
            let result = self
                .client
                .tx()
                .sign_and_submit(&reward_tx, &self.signer, tx_params)
                .await.unwrap();
            println!("Deposit Transaction {:?}", result);
            Ok(())
        }
    }