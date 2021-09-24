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
use node_polkadex_runtime::{SessionKeys};
use node_polkadex_runtime::{
    wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig,
    CouncilConfig, IndicesConfig,
    OrmlVestingConfig, SessionConfig, StakerStatus, PDEXMigrationConfig,PolkadexOcexConfig,
    StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig, TokensConfig
};
use log::info;
pub use node_polkadex_runtime::GenesisConfig;
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
    let boot_nodes = vec![];
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
    let root_key = hex!["a2f6199c36e89dd066de2582ae6f705783f040c1fc06f30455532258886bfa76"].into();
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
        tokens: TokensConfig {
            balances: vec![],
        },
        pdex_migration: PDEXMigrationConfig {
            max_tokens: ERC20_PDEX_SUPPLY,
            operation_status: false
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
        (hex!["e4cdc8abc0405db44c1a6886a2f2c59012fa3b98c07b61d63cc7f9e437ba243e"].into(), 3 * 6_000 * PDEX),
        (hex!["b26562a2e476fea86b26b2e47f12d279deb0ca7812bd1dad5b4fc8a909e10b22"].into(), 3 * 800_000 * PDEX),
        (hex!["e83adffb6338272e981cbc0c6cc03fd4e5e8447497b6b531b9436870c6079758"].into(), 3 * 6_000 * PDEX),
        (hex!["e613dd948e7baacc02c97db737ad43af7024f5ae595d06f1611ce827c300b17f"].into(), 3 * 120_000 * PDEX),
        (hex!["182400644f4780a65a43e00f9630152fe0ab2323d0dacd04e808ceccf462f416"].into(), 3 * 35_000 * PDEX),
        (hex!["b8779ddd7bc8dc00dc0e220b6b07b509553c3cdbdad3e384cc1ba2187cbca53f"].into(), 3 * 1875 * PDEX),
        (hex!["62168680c9ed6e456fa59bd01525a53dd6fa991757e920482016e7db6caebd45"].into(), 3 * 4375 * PDEX),
        (hex!["1c2eaec3bd844d93d29d442c1ecc431e8502ce7b13f950ae203ee63ff2a1750a"].into(), 3 * 17500 * PDEX),
        (hex!["78d4caac9c5b562190901aafb9f2c74780c5831a89257254ca225729e755d919"].into(), 3 * 17500 * PDEX),
        (hex!["1a8538e949213a4034bca131957bbfe8bc45107be4e93c86f92353fccff90039"].into(), 3 * 17500 * PDEX),
        (hex!["fac6591fd5605154f1a77fc142d66d9c2f7b11f5c0bc61a3ac8ab46099e87e3a"].into(), 3 * 5250 * PDEX),
        (hex!["ccb97ce4726461ad53c0ec9277b1ba3f7f88b0a11f847f1ca17d358e6e4d0a05"].into(), 3 * 63750 * PDEX),
        (hex!["08a1c86a2c789eeb1295c3b3ba63b2cde5d23fa6c80d8f87246c21a11fa3ba1d"].into(), 3 * 35000 * PDEX),
        (hex!["082cb53d6299dc033e467de007bfd5c4c0d24135aa85d2f1d983008ff78fbb66"].into(), 3 * 42500 * PDEX),
        (hex!["48cb52f3831917977aec38d9c3a3c73c8253b82523af35d44b7122e674677f05"].into(), 3 * 17500 * PDEX),
        (hex!["0617b168a08acd31e3323ff63cb6e8e7682ba002ca0184a59a0ebc6dcf4e7f2b"].into(), 3 * 17500 * PDEX),

        (hex!["0a1f6fa0345ceac40338c78bdfc68a211898921032d30e1b4492090c29962505"].into(), 3 * 10625 * PDEX),
        (hex!["b2fa882baef6358e3b4379c290fc989093da5f62b0c8cc57bb972fa7232efe10"].into(), 3 * 8750 * PDEX),
        (hex!["ecd0a0fba2f97d02d81fa3408e7e1f4a40b36d58fb7b999f0d0f5e073b810d3d"].into(), 3 * 31875 * PDEX),
        (hex!["0838d06bad89b000120bea3e2cbf59e342f518a3f76becfa8c35bfd386e79825"].into(), 3 * 17500 * PDEX),
        (hex!["60285b86e8196e4e20565440e2ded16459a8f1e8b6c5ce8bacb4a5b11eee8b05"].into(), 3 * 25250 * PDEX),
        (hex!["68732830b518f410592bfb6f623e9864e9c021bc4adfe4845916932024bf9119"].into(), 3 * 2625 * PDEX),
        (hex!["bc13c9a902a524609f064014695f2b6548a17d7e8bb12a834220559bc38bbc5d"].into(), 3 * 14000 * PDEX),
        (hex!["daeb89c994d06f7e996e2c3e9e1fe685765e40f083432fbcdcb7f77bc1f9a378"].into(), 3 * 2625 * PDEX),
        (hex!["3ceab1c17a4302ac0471e943279bd993adf12af6d2010a4f73bbdf428fba914f"].into(), 3 * 10000 * PDEX),
        (hex!["baf1346f012c29003aeb63ac2503fbfafcd0dc182e98053b34f8bb08510ca73f"].into(), 3 * 15280 * PDEX),
        (hex!["969554a9c50959bc434b99051b9803cc911ba3cad6c0e1d2ab2b8bcbbd1f057e"].into(), 3 * 20000 * PDEX),

        (hex!["724513af8211cbaaeb17e7bbff8f2286718135d4ebe10e556c5b2076dbbd342d"].into(), 3 * 20000 * PDEX),
        (hex!["cc0056b00683900613556f57c5324a2882fa9b5f50702e61ffade0b1102f0674"].into(), 3 * 10000 * PDEX),
        (hex!["eab1d6b0efce910517067712d026e42ab5f84ffd068b80d3cd55cd7c95d4db68"].into(), 3 * 20000 * PDEX),
        (hex!["3ee90311650ce54b81d70f77537dc255c130ac9f5f5933cc6e2cedcb00ebdf5d"].into(), 3 * 50000 * PDEX),
        (hex!["a0cc2a61879f21b7924392cfea5c35b47781f795ca24d179188c6d3f2a67952b"].into(), 3 * 20000 * PDEX),
        (hex!["2c6ce334da34c1ffdfb9cfb9962afdc9decf8f36b8d5282c2dbdef7c7b1aee53"].into(), 3 * 20000 * PDEX),
        (hex!["aa36b0d46767a839e11f18d8f15d373ed1f63abb33324edd87ebdc5fcfabd812"].into(), 3 * 20000 * PDEX),

        (hex!["7a56462554bef5d4f946a3c2ea1798398303aaf49e2d80d272096fb04cd95d06"].into(), 3 * 375 * PDEX),
        (hex!["9a82629aac0895e5998542537f6b5b3a1c2c6fd46e827d409de88aacf9755a0e"].into(), 3 * 938 * PDEX),
        (hex!["8039b9f35380bc3c20206d25c44006bd98e1252d7cb80acd6290b4f9c17bcd4c"].into(), 50000 * PDEX),
        (hex!["ec3cfd6b94a36adf49492caae5c59005b04e88a936c6106c4feca1631b5d6025"].into(), 50000 * PDEX),
        (hex!["8a442ebbcdb3aeace616292a957f36462e1e4c69e11de340527bfb617b01e068"].into(), 50000 * PDEX),
        (hex!["2c6789aa288e153564fe1ad4f824d8b760171db53d4e7500e2d3f9d51e979e03"].into(), 400000 * PDEX),
        (hex!["148d5e55a937b6a6c80db86b28bc55f7336b17b13225e80468eef71d01c79341"].into(), 3655828 * PDEX),
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
