use grandpa_primitives::AuthorityId as GrandpaId;
use hex_literal::hex;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use polkadex_primitives::Block;
pub use polkadex_primitives::{AccountId, Balance, Signature};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{traits::{IdentifyAccount, Verify}, Perbill};
pub use node_polkadex_runtime::GenesisConfig;
use node_polkadex_runtime::{
    wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig,
    CouncilConfig, IndicesConfig,
    // ElectionsConfig, GrandpaConfig, ImOnlineConfig, 
    OrmlVestingConfig, SessionConfig, SessionKeys, StakerStatus, PDEXMigrationConfig,PolkadexOcexConfig,
    StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig, TokensConfig
};
use log::info;
use node_polkadex_runtime::constants::currency::PDEX;
use sp_runtime::traits::AccountIdConversion;
use frame_support::PalletId;

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";
const MAINNET_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,

}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;


pub(crate) fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
    im_online: ImOnlineId,
    authority_discovery: AuthorityDiscoveryId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        babe,
        im_online,
        authority_discovery,
    }
}

fn udon_testnet_config_genesis() -> GenesisConfig {
    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![
        (
            // 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
            hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
            // 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
            hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
            // 5H4dmeetCqvLhFbXCQ9MrfHCu7RatJHrPYD71Jikdnt2VZeD
            hex!["dd1f6928c778a52b605889acb99d627b3a9be9a42439c77bc00f1980d4f540ec"]
                .unchecked_into(),
            // 5EynamEisSmW3kUdGC7BSXQy1oR8rD1CWLjHh2LGz8bys3sg
            hex!["80f461b74b90b4913e0354569e90c7cd11ca5dbce6e8b2a6fcbbe0761b877e06"]
                .unchecked_into(),
            // 5EynamEisSmW3kUdGC7BSXQy1oR8rD1CWLjHh2LGz8bys3sg
            hex!["80f461b74b90b4913e0354569e90c7cd11ca5dbce6e8b2a6fcbbe0761b877e06"]
                .unchecked_into(),
            // 5EynamEisSmW3kUdGC7BSXQy1oR8rD1CWLjHh2LGz8bys3sg
            hex!["80f461b74b90b4913e0354569e90c7cd11ca5dbce6e8b2a6fcbbe0761b877e06"]
                .unchecked_into(),
        ),
        (
            // 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
            hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
            // 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
            hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
            // 5H85GsLD6svD6PHtpenjiXVyHGcwCCYB8zbdrVDPWsuocDYB
            hex!["dfbf0015a3b9e483606f595ea122b3f2355b46d9085fcb0639cb03f05467ab59"]
                .unchecked_into(),
            // 5GC5FgdZbCYkMnZ2Ez8o2zztvkdR3qn1Zymknbi97vUsk2vV
            hex!["b68fae03e44288bde5c66fd89893d943baf88b8cffb33aa7f1dedf0d4a86ad3c"]
                .unchecked_into(),
            // 5GC5FgdZbCYkMnZ2Ez8o2zztvkdR3qn1Zymknbi97vUsk2vV
            hex!["b68fae03e44288bde5c66fd89893d943baf88b8cffb33aa7f1dedf0d4a86ad3c"]
                .unchecked_into(),
            // 5GC5FgdZbCYkMnZ2Ez8o2zztvkdR3qn1Zymknbi97vUsk2vV
            hex!["b68fae03e44288bde5c66fd89893d943baf88b8cffb33aa7f1dedf0d4a86ad3c"]
                .unchecked_into(),
        ),
    ];

    let root_key: AccountId = hex![
        // 5Ggr5JRSxCSZvwTc9Xkjca5bWkkmG1btufW22uLm5tArfV9y
        "cc816e946438b2b21b8a3073f983ce03ee0feb313ec494e2dec462cfb4e77502"
    ]
        .into();

    testnet_genesis(
        initial_authorities,
        vec![],
        root_key
    )
}

/// Staging testnet config.
pub fn udon_testnet_config() -> ChainSpec {
    let boot_nodes = vec![
          "/ip4/13.235.190.203/tcp/30333/p2p/12D3KooWMJ4AMmzpRbv914ZGZR6ehBhcZvGtqYid5jxSx8vXiSr7".parse().unwrap(),
          "/ip4/54.176.87.85/tcp/30333/ws/p2p/12D3KooWQfzQ5eJfXxJ1GK2xKNYhDQGyEvt3cteokENHd6rnSFJC".parse().unwrap(),
          "/ip4/18.198.113.243/tcp/30333/ws/p2p/12D3KooWHzoUKuYehN51LNc1QYK8er4KyweikD1GQaZn9ViCkygN".parse().unwrap(),
          "/ip4/52.28.14.93/tcp/30333/p2p/12D3KooWE1t2sSVntkhpFRva6UGdXJKeDqMfjtRYFGX8msHfEdZi".parse().unwrap()
    ];
    ChainSpec::from_genesis(
        "Polkadex Test Net",
        "polkadex_udon_testnet",
        ChainType::Live,
        udon_testnet_config_genesis,
        boot_nodes,
        Some(
            TelemetryEndpoints::new(vec![(STAGING_TELEMETRY_URL.to_string(), 0)])
                .expect("Staging telemetry url is valid; qed"),
        ),
        None,
        None,
        Default::default(),
    )
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
    where
        AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
    seed: &str,
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
    ImOnlineId,
    AuthorityDiscoveryId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
        get_from_seed::<ImOnlineId>(seed),
        get_from_seed::<AuthorityDiscoveryId>(seed),
    )
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice")
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}

fn soba_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice")
    )
}

/// Local testnet config ()
pub fn soba_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "soba_testnet",
        ChainType::Local,
        soba_testnet_genesis,
        vec![],
        None,
        None,
        None,
        Default::default(),
    )
}


fn mainnet_genesis_constuctor() -> GenesisConfig {
    // TODO: Change this with our validator accounts before launch
    let initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )> = vec![
        (
            // 5Fbsd6WXDGiLTxunqeK5BATNiocfCqu9bS1yArVjCgeBLkVy
            hex!["9c7a2ee14e565db0c69f78c7b4cd839fbf52b607d867e9e9c5a79042898a0d12"].into(),
            // 5EnCiV7wSHeNhjW3FSUwiJNkcc2SBkPLn5Nj93FmbLtBjQUq
            hex!["781ead1e2fa9ccb74b44c19d29cb2a7a4b5be3972927ae98cd3877523976a276"].into(),
            // 5H4dmeetCqvLhFbXCQ9MrfHCu7RatJHrPYD71Jikdnt2VZeD
            hex!["dd1f6928c778a52b605889acb99d627b3a9be9a42439c77bc00f1980d4f540ec"]
                .unchecked_into(),
            // 5EynamEisSmW3kUdGC7BSXQy1oR8rD1CWLjHh2LGz8bys3sg
            hex!["80f461b74b90b4913e0354569e90c7cd11ca5dbce6e8b2a6fcbbe0761b877e06"]
                .unchecked_into(),
            // 5EynamEisSmW3kUdGC7BSXQy1oR8rD1CWLjHh2LGz8bys3sg
            hex!["80f461b74b90b4913e0354569e90c7cd11ca5dbce6e8b2a6fcbbe0761b877e06"]
                .unchecked_into(),
            // 5EynamEisSmW3kUdGC7BSXQy1oR8rD1CWLjHh2LGz8bys3sg
            hex!["80f461b74b90b4913e0354569e90c7cd11ca5dbce6e8b2a6fcbbe0761b877e06"]
                .unchecked_into(),
        ),
        (
            // 5ERawXCzCWkjVq3xz1W5KGNtVx2VdefvZ62Bw1FEuZW4Vny2
            hex!["68655684472b743e456907b398d3a44c113f189e56d1bbfd55e889e295dfde78"].into(),
            // 5Gc4vr42hH1uDZc93Nayk5G7i687bAQdHHc9unLuyeawHipF
            hex!["c8dc79e36b29395413399edaec3e20fcca7205fb19776ed8ddb25d6f427ec40e"].into(),
            // 5H85GsLD6svD6PHtpenjiXVyHGcwCCYB8zbdrVDPWsuocDYB
            hex!["dfbf0015a3b9e483606f595ea122b3f2355b46d9085fcb0639cb03f05467ab59"]
                .unchecked_into(),
            // 5GC5FgdZbCYkMnZ2Ez8o2zztvkdR3qn1Zymknbi97vUsk2vV
            hex!["b68fae03e44288bde5c66fd89893d943baf88b8cffb33aa7f1dedf0d4a86ad3c"]
                .unchecked_into(),
            // 5GC5FgdZbCYkMnZ2Ez8o2zztvkdR3qn1Zymknbi97vUsk2vV
            hex!["b68fae03e44288bde5c66fd89893d943baf88b8cffb33aa7f1dedf0d4a86ad3c"]
                .unchecked_into(),
            // 5GC5FgdZbCYkMnZ2Ez8o2zztvkdR3qn1Zymknbi97vUsk2vV
            hex!["b68fae03e44288bde5c66fd89893d943baf88b8cffb33aa7f1dedf0d4a86ad3c"]
                .unchecked_into(),
        ), ];
    let root_key = hex!["70a5f4e786b47baf52d5a34742bb8312139cfe1c747fbeb3912c197d38c53332"].into();
    testnet_genesis(initial_authorities, vec![], root_key)
}

pub fn mainnet_testnet_config() -> ChainSpec {
    let bootnodes = vec![];
    const POLKADEX_PROTOCOL_ID: &str = "pdex";
    ChainSpec::from_genesis(
        "Polkadex Main Network",
        "polkadex_main_network",
        ChainType::Live,
        mainnet_genesis_constuctor,
        bootnodes,
        Some(
            TelemetryEndpoints::new(vec![(MAINNET_TELEMETRY_URL.to_string(), 0)])
                .expect("Staging telemetry url is valid; qed"),
        ),
        Some(POLKADEX_PROTOCOL_ID),
        None,
        Default::default(),
    )
}

use itertools::Itertools;
use polkadex_primitives::assets::AssetId;

fn adjust_treasury_balance_for_initial_validators(initial_validators: usize, endowment: u128) -> u128 {
    // The extra one is for root_key
    (initial_validators + 1) as u128 * endowment
}

#[allow(non_upper_case_globals)]
pub const OCEXGenesisAccount: PalletId = PalletId(*b"polka/ga");

/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
        ImOnlineId,
        AuthorityDiscoveryId,
    )>,
    _initial_nominators: Vec<AccountId>,
    root_key: AccountId
) -> GenesisConfig {
    const ENDOWMENT: u128 = 20_000 * PDEX;
    const STASH: u128 = 2 * PDEX;
    let genesis: AccountId = OCEXGenesisAccount.into_account();
    // Total Supply in ERC20
    const ERC20_PDEX_SUPPLY: u128 = 3_172_895 * PDEX;
    // Total funds in treasury also includes 2_000_000 PDEX for parachain auctions
    let mut treasury_funds: u128 = 10_200_000 * PDEX;
    treasury_funds = treasury_funds - adjust_treasury_balance_for_initial_validators(initial_authorities.len(), ENDOWMENT);

    info!("🧮 Tokens taken from treasury:  {:>22}",adjust_treasury_balance_for_initial_validators(initial_authorities.len(), ENDOWMENT));
    info!("🧮 Token remaining in treasury: {:>22}",treasury_funds);
    // Treasury Account Id
    pub const TREASURY_PALLET_ID: PalletId = PalletId(*b"py/trsry");
    let treasury_account: AccountId = TREASURY_PALLET_ID.into_account();

    let mut inital_validators_endowment = initial_authorities
        .iter()
        .map(|k| (k.0.clone(), ENDOWMENT)).collect_vec();
    let mut endowed_accounts = vec![
        //      Root Key
        (root_key.clone(), ENDOWMENT),
        //     Treasury Funds
        (treasury_account, treasury_funds),
    ];
    // Get rest of the stake holders
    let mut claims = get_stakeholder_tokens();

    let mut total_claims: u128 = 0;
    for (_, balance) in &claims {
        total_claims = total_claims + balance;
    }

    info!("🧮 Total Investor Tokens:       {:>22}",total_claims);
    // assert_eq!(total_claims, 6_627_105 * PDEX, "Total claims is configured correctly");

    endowed_accounts.append(claims.as_mut());
    // Endow to validators
    endowed_accounts.append(&mut inital_validators_endowment);

    let mut total_supply: u128 = 0;
    for (_, balance) in &endowed_accounts {
        total_supply = total_supply + balance.clone()
    }

    info!("🧮  Assert Total supply is 20 million: {} == {} ", total_supply + ERC20_PDEX_SUPPLY , 20_000_000 * PDEX);

    let vesting = get_vesting_terms();


    GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        balances: BalancesConfig {
            balances: endowed_accounts,
        },

        indices: IndicesConfig { indices: vec![] },
        session: SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(),
                                     x.3.clone(),
                                     x.4.clone(),
                                     x.5.clone(),
                        ),
                    )
                })
                .collect::<Vec<_>>(),
        },
        staking: StakingConfig {
            minimum_validator_count: 1,
            validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities
                .iter()
                .map(|x| x.0.clone()).collect(),
            stakers: initial_authorities
                .iter()
                .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
                .collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            ..Default::default()
        },
        elections: Default::default(),
        council: CouncilConfig { members: vec![], phantom: Default::default() },
        technical_committee: TechnicalCommitteeConfig {
            members: vec![],
            phantom: Default::default(),
        },
        sudo: SudoConfig {
            key: root_key.clone(),
        },
        babe: BabeConfig {
            authorities: Default::default(),
            epoch_config: Some(node_polkadex_runtime::BABE_GENESIS_EPOCH_CONFIG),
        },
        im_online: Default::default(),
        authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
        grandpa: Default::default(),
        technical_membership: Default::default(),
        treasury: Default::default(),
        orml_vesting: OrmlVestingConfig { vesting },

        pdex_migration: PDEXMigrationConfig {
            max_tokens: ERC20_PDEX_SUPPLY,
            operation_status: false
        },
    	tokens: TokensConfig {
            balances :  vec![(root_key.clone(),AssetId::Asset(24),1000000u128)],
            // [
            //     (endowed_accounts[0].to_owned(), AssetId::Asset(24), 1000000000000000000u128),
            //     (endowed_accounts[0].to_owned(), AssetId::Asset(702), 1000000000000000000u128),

            // ],
        },
        polkadex_ocex: PolkadexOcexConfig {
            key: genesis.clone(),
            genesis_account: genesis,
        },
    }
}

pub fn get_vesting_terms() -> Vec<(AccountId, u32, u32, u32, Balance)> {
    // 3 months in terms of 12s blocks is 648,000 blocks, i.e. period = 648,000
    // TODO:
    // who, start, period, period_count, per_period
    // vec![ (hex!["148d5e55a937b6a6c80db86b28bc55f7336b17b13225e80468eef71d01c79341"].into(), 1, 30, 1, 3655828 * PDEX)]
    vec![]
}

pub fn get_stakeholder_tokens() -> Vec<(AccountId, Balance)> {
    let claims = vec![

    ];
    claims
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            vec![],
            get_account_id_from_seed::<sr25519::Public>("Alice")
        )
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            soba_testnet_genesis,
            vec![],
            None,
            None,
            None,
            Default::default(),
        )
    }

    #[test]
    fn test_create_development_chain_spec() {
        assert!(!development_config().build_storage().is_err());
    }

    #[test]
    fn test_create_soba_testnet_chain_spec() {
        assert!(!soba_testnet_config().build_storage().is_err());
    }

    #[test]
    fn test_staging_test_net_chain_spec() {
        assert!(!udon_testnet_config().build_storage().is_err());
    }
}
