use sp_api::{ApiRef, ProvideRuntimeApi};
use sp_core::Pair;
use sp_keyring::AccountKeyring;

use bls_primitives::Pair as BLSPair;
use orderbook_primitives::crypto::AuthorityId;
use orderbook_primitives::ObApi;
use orderbook_primitives::SnapshotSummary;
use orderbook_primitives::ValidatorSet;
use polkadex_primitives::{AccountId, Block};

pub(crate) fn make_ob_ids(keys: &[AccountKeyring]) -> Vec<AuthorityId> {
    SnapshotSummary::default();
    keys.iter().map(|key| {
        let seed = key.to_seed();
        println!("Loaded seed: {}", seed);
        BLSPair::from_string(&seed, None).unwrap().public().into()
    }).collect()
}

macro_rules! create_test_api {
    ( $api_name:ident, latest_summary: $latest_summary:expr,ingress_messages: $ingress_messages:expr, $($inits:expr),+ ) => {
		pub(crate) mod $api_name {
			use super::*;

			#[derive(Clone, Default)]
			pub(crate) struct TestApi {}

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
                impl ObApi<Block> for RuntimeApi {
                    /// Return the current active Orderbook validator set
					fn validator_set() -> ValidatorSet<AuthorityId>{ValidatorSet::new(make_ob_ids(&[$($inits),+]), 0).unwrap()}

					fn get_latest_snapshot() -> SnapshotSummary{$latest_summary}

					/// Return the ingress messages at the given block
					fn ingress_messages() -> Vec<polkadex_primitives::ingress::IngressMessages<AccountId>>{$ingress_messages}

					/// Submits the snapshot to runtime
					fn submit_snapshot(_: SnapshotSummary) -> Result<(), ()>{Ok(())}
                }
			}
		}
	};
}

create_test_api!(
	two_validators,
	latest_summary: SnapshotSummary::default(),
	ingress_messages: vec![],
	AccountKeyring::Alice,
	AccountKeyring::Bob
);


