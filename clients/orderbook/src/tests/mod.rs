pub mod sync;

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use primitive_types::H256;
use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::AccountId32;
use sp_core::ecdsa::Public;
use orderbook_primitives::{ObApi, SnapshotSummary, ValidatorSet};
use orderbook_primitives::crypto::AuthorityId;
use orderbook_primitives::types::TradingPair;
use polkadex_primitives::{AccountId, Block, BlockNumber};
use polkadex_primitives::ocex::TradingPairConfig;
use polkadex_primitives::withdrawal::Withdrawal;

#[derive(Clone, Default)]
pub(crate) struct TestApi {
    pub active: Vec<AuthorityId>,
    pub latest_snapshot_nonce: Arc<RwLock<u64>>,
    pub snapshots: Arc<RwLock<HashMap<u64, SnapshotSummary<AccountId>>>>,
    pub unprocessed: Arc<RwLock<HashMap<(u64, H256), SnapshotSummary<AccountId>>>>,
    pub main_to_proxy_mapping: HashMap<AccountId, Vec<AccountId>>,
    pub pending_snapshot: Option<u64>,
    pub operator_key: Option<Public>,
    pub trading_config: Vec<(TradingPair, TradingPairConfig)>,
    pub withdrawals: Arc<RwLock<HashMap<u64, Vec<Withdrawal<AccountId>>>>>
}

impl TestApi {
    pub fn validator_set(&self) -> ValidatorSet<AuthorityId> {
        ValidatorSet { validators: self.active.clone() }
    }

    pub fn get_latest_snapshot(&self) -> SnapshotSummary<AccountId> {
        self.snapshots.read().get(&*self.latest_snapshot_nonce.read())
            .unwrap_or(&SnapshotSummary{
                snapshot_id: 0,
                state_root: Default::default(),
                state_change_id: 0,
                state_chunk_hashes: vec![],
                bitflags: vec![],
                withdrawals: vec![],
                aggregate_signature: None,
            }).clone()
    }

    pub fn submit_snapshot(&self, snapshot: SnapshotSummary<AccountId>) -> Result<(), ()> {
        assert_eq!(self.latest_snapshot_nonce.read().saturating_add(1), snapshot.snapshot_id);
        let summary_hash = H256::from_slice(&snapshot.sign_data());
        let working_summary = match self
            .unprocessed.read().get(&(snapshot.snapshot_id, summary_hash)).cloned() {
            None => snapshot,
            Some(mut stored_summary) => {
                let signature = snapshot.aggregate_signature.unwrap();
                let auth_index = snapshot.signed_auth_indexes().first().unwrap().clone();
                // Verify the auth signature.
                let signer: &AuthorityId = self.active.get(auth_index as usize).unwrap();
                assert!(signer.verify(&snapshot.sign_data(),&signature.into()));
                // Aggregate signature
                assert!(stored_summary.add_signature(signature).is_ok());
                // update the bitfield
                stored_summary.add_auth_index(auth_index);
                stored_summary.clone()
            }
        };


        let total_validators = self.active.len();
        if working_summary.signed_auth_indexes().len() >= total_validators.saturating_mul(2).saturating_div(3) {
            self.unprocessed.write().remove(&(working_summary.snapshot_id,summary_hash));
            let withdrawals = working_summary.withdrawals.clone();
            let mut withdrawals_map = self.withdrawals.write();
            withdrawals_map.insert(working_summary.snapshot_id,withdrawals);
            *self.latest_snapshot_nonce.write() = working_summary.snapshot_id;
            let mut snapshots = self.snapshots.write();
            snapshots.insert(working_summary.snapshot_id,working_summary);
        }else{
            let mut unprocessed = self.unprocessed.write();
            unprocessed.insert((working_summary.snapshot_id,summary_hash),working_summary);
        }

        Ok(())
    }

    pub fn get_all_accounts_and_proxies(&self) -> Vec<(AccountId, Vec<AccountId>)> {
        self.main_to_proxy_mapping.iter().map( |(k,v) | {
            (k.clone(),v.clone())
        }).collect()
    }

    pub fn get_snapshot_generation_intervals(&self) -> (u64, u32) {
        (20, 5)
    }

    pub fn pending_snapshot(&self) -> Option<u64> {
        self.pending_snapshot
    }

    pub fn get_orderbook_opearator_key(&self) -> Option<Public> {
        self.operator_key
    }

    pub fn get_last_accepted_stid(&self) -> u64 {
        self.snapshots.read().get(&*self.latest_snapshot_nonce.read()).unwrap_or(&SnapshotSummary{
            snapshot_id: 0,
            state_root: Default::default(),
            state_change_id: 0,
            state_chunk_hashes: vec![],
            bitflags: vec![],
            withdrawals: vec![],
            aggregate_signature: None,
        }).state_change_id
    }

    pub fn read_trading_pair_configs(&self) -> Vec<(TradingPair, TradingPairConfig)> {
        self.trading_config.clone()
    }
}

sp_api::mock_impl_runtime_apis! {
impl ObApi<Block> for RuntimeApi {
    /// Return the current active Orderbook validator set
    fn validator_set() -> ValidatorSet<AuthorityId>
    {
        self.inner.validator_set()
    }

    fn get_latest_snapshot() -> SnapshotSummary<AccountId> {
        self.inner.snapshots.read().get(&*self.inner.latest_snapshot_nonce.read()).unwrap().clone()
    }

    /// Return the ingress messages at the given block
    fn ingress_messages() -> Vec<polkadex_primitives::ingress::IngressMessages<AccountId>> { Vec::new() }

    /// Submits the snapshot to runtime
    fn submit_snapshot(summary: SnapshotSummary<AccountId>) -> Result<(), ()> {
            self.inner.submit_snapshot(summary)
        }

    /// Get Snapshot By Id
    fn get_snapshot_by_id(id: u64) -> Option<SnapshotSummary<AccountId>> {
            self.inner.snapshots.read().get(&id).cloned()
        }

    /// Returns all main account and corresponding proxies at this point in time
    fn get_all_accounts_and_proxies() -> Vec<(AccountId, Vec<AccountId>)> {
            self.inner.get_all_accounts_and_proxies()
        }

    /// Returns snapshot generation intervals
    fn get_snapshot_generation_intervals() -> (u64, BlockNumber) {
            self.inner.get_snapshot_generation_intervals()
        }

		/// Gets pending snapshot if any
		fn pending_snapshot() -> Option<u64>{
            self.inner.pending_snapshot()
        }

		/// Returns Public Key of Whitelisted Orderbook Operator
		fn get_orderbook_opearator_key() -> Option<sp_core::ecdsa::Public>{
            self.inner.get_orderbook_opearator_key()
        }


		/// Returns last processed stid from last snapshot
		fn get_last_accepted_stid() -> u64{
            self.inner.get_last_accepted_stid()
        }

		/// Reads the current trading pair configs
		fn read_trading_pair_configs() -> Vec<(TradingPair, TradingPairConfig)>{
            self.inner.read_trading_pair_configs()
        }
}
}


// compiler gets confused and warns us about unused inner
#[allow(dead_code)]
pub(crate) struct RuntimeApi {
    inner: TestApi,
}

impl ProvideRuntimeApi<Block> for TestApi {
    type Api = RuntimeApi;
    fn runtime_api(&self) -> ApiRef<Self::Api> {
        RuntimeApi { inner: self.clone() }.into()
    }
}