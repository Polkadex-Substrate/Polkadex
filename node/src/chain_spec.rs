use frame_benchmarking::frame_support::PalletId;
use grandpa_primitives::AuthorityId as GrandpaId;
use hex_literal::hex;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use polkadex_primitives::assets::AssetId;
use polkadex_primitives::Block;
pub use polkadex_primitives::{AccountId, Balance, Signature};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;
use sc_telemetry::TelemetryEndpoints;
use serde::{Deserialize, Serialize};
use sp_authority_discovery::AuthorityId as AuthorityDiscoveryId;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{crypto::UncheckedInto, sr25519, Pair, Public};
use sp_runtime::{
    traits::{AccountIdConversion, IdentifyAccount, Verify},
    Perbill,
};

use node_polkadex_runtime::constants::currency::*;
pub use node_polkadex_runtime::GenesisConfig;
use node_polkadex_runtime::{
    wasm_binary_unwrap, AuthorityDiscoveryConfig, BabeConfig, BalancesConfig, ContractsConfig,
    CouncilConfig, ElectionsConfig, GrandpaConfig, ImOnlineConfig, IndicesConfig,
    OrmlVestingConfig, PolkadexTreasuryModuleId, SessionConfig, SessionKeys, StakerStatus,
    StakingConfig, SudoConfig, SystemConfig, TechnicalCommitteeConfig, TokensConfig,
    MAX_NOMINATIONS,
};

type AccountPublic = <Signature as Verify>::Signer;

const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;
// /// Flaming Fir testnet generator
// pub fn flaming_fir_config() -> Result<ChainSpec, String> {
//     ChainSpec::from_json_bytes(&include_bytes!("../res/flaming-fir.json")[..])
// }

fn session_keys(
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
    // stash, controller, session-key
    // generated with secret:
    // for i in 1 2 3 4 ; do for j in stash controller; do subkey inspect "$secret"/fir/$j/$i; done; done
    // and
    // for i in 1 2 3 4 ; do for j in session; do subkey --ed25519 inspect "$secret"//fir//$j//$i; done; done

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

    // generated with secret: subkey inspect "$secret"/fir
    let root_key: AccountId = hex![
        // 5Ggr5JRSxCSZvwTc9Xkjca5bWkkmG1btufW22uLm5tArfV9y
        "cc816e946438b2b21b8a3073f983ce03ee0feb313ec494e2dec462cfb4e77502"
    ]
    .into();
    // this is the accouint
    let endowed_accounts: Vec<AccountId> = vec![root_key.clone()];

    testnet_genesis(
        initial_authorities,
        vec![],
        root_key,
        Some(endowed_accounts),
        false,
    )
}

/// Staging testnet config.
pub fn udon_testnet_config() -> ChainSpec {
    let boot_nodes = vec![];
    ChainSpec::from_genesis(
        "Staging Testnet",
        "udon_testnet",
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
    initial_nominators: Vec<AccountId>,
    root_key: AccountId,
    endowed_accounts: Option<Vec<AccountId>>,
    enable_println: bool,
) -> GenesisConfig {
    let _genesis: AccountId = OCEXGenesisAccount.into_account();
    let treasury_accont: AccountId = PolkadexTreasuryModuleId::get().into_account();
    let mut investor_balances = vec![
       (hex!["e4cdc8abc0405db44c1a6886a2f2c59012fa3b98c07b61d63cc7f9e437ba243e"].into(), 18000050000000000),
       (hex!["b26562a2e476fea86b26b2e47f12d279deb0ca7812bd1dad5b4fc8a909e10b22"].into(), 240000050000000000),
       (hex!["e83adffb6338272e981cbc0c6cc03fd4e5e8447497b6b531b9436870c6079758"].into(), 18000050000000000),
       (hex!["e613dd948e7baacc02c97db737ad43af7024f5ae595d06f1611ce827c300b17f"].into(), 360000050000000000),
       (hex!["182400644f4780a65a43e00f9630152fe0ab2323d0dacd04e808ceccf462f416"].into(), 105000050000000000),
       (hex!["b8779ddd7bc8dc00dc0e220b6b07b509553c3cdbdad3e384cc1ba2187cbca53f"].into(), 5625050000000000),
       (hex!["62168680c9ed6e456fa59bd01525a53dd6fa991757e920482016e7db6caebd45"].into(), 13125050000000000),
       (hex!["1c2eaec3bd844d93d29d442c1ecc431e8502ce7b13f950ae203ee63ff2a1750a"].into(), 52500050000000000),
       (hex!["78d4caac9c5b562190901aafb9f2c74780c5831a89257254ca225729e755d919"].into(), 52500050000000000),
       (hex!["1a8538e949213a4034bca131957bbfe8bc45107be4e93c86f92353fccff90039"].into(), 52500050000000000),
       (hex!["fac6591fd5605154f1a77fc142d66d9c2f7b11f5c0bc61a3ac8ab46099e87e3a"].into(), 15750050000000000),
       (hex!["ccb97ce4726461ad53c0ec9277b1ba3f7f88b0a11f847f1ca17d358e6e4d0a05"].into(), 191250050000000000),
       (hex!["08a1c86a2c789eeb1295c3b3ba63b2cde5d23fa6c80d8f87246c21a11fa3ba1d"].into(), 105000050000000000),
       (hex!["082cb53d6299dc033e467de007bfd5c4c0d24135aa85d2f1d983008ff78fbb66"].into(), 127500050000000000),
       (hex!["48cb52f3831917977aec38d9c3a3c73c8253b82523af35d44b7122e674677f05"].into(), 52500050000000000),
       (hex!["0617b168a08acd31e3323ff63cb6e8e7682ba002ca0184a59a0ebc6dcf4e7f2b"].into(), 52500050000000000),
       (hex!["b2fa882baef6358e3b4379c290fc989093da5f62b0c8cc57bb972fa7232efe10"].into(), 26250050000000000),
       (hex!["ecd0a0fba2f97d02d81fa3408e7e1f4a40b36d58fb7b999f0d0f5e073b810d3d"].into(), 95625050000000000),
       (hex!["0838d06bad89b000120bea3e2cbf59e342f518a3f76becfa8c35bfd386e79825"].into(), 52500050000000000),
       (hex!["60285b86e8196e4e20565440e2ded16459a8f1e8b6c5ce8bacb4a5b11eee8b05"].into(), 75750050000000000),
       (hex!["68732830b518f410592bfb6f623e9864e9c021bc4adfe4845916932024bf9119"].into(), 7875050000000000),
       (hex!["bc13c9a902a524609f064014695f2b6548a17d7e8bb12a834220559bc38bbc5d"].into(), 42000050000000000),
       (hex!["daeb89c994d06f7e996e2c3e9e1fe685765e40f083432fbcdcb7f77bc1f9a378"].into(), 7875050000000000),
       (hex!["3ceab1c17a4302ac0471e943279bd993adf12af6d2010a4f73bbdf428fba914f"].into(), 30000050000000000),
       (hex!["baf1346f012c29003aeb63ac2503fbfafcd0dc182e98053b34f8bb08510ca73f"].into(), 45840050000000000),
       (hex!["969554a9c50959bc434b99051b9803cc911ba3cad6c0e1d2ab2b8bcbbd1f057e"].into(), 60000050000000000),
       (hex!["724513af8211cbaaeb17e7bbff8f2286718135d4ebe10e556c5b2076dbbd342d"].into(), 60000050000000000),
       (hex!["eab1d6b0efce910517067712d026e42ab5f84ffd068b80d3cd55cd7c95d4db68"].into(), 60000050000000000),
       (hex!["3ee90311650ce54b81d70f77537dc255c130ac9f5f5933cc6e2cedcb00ebdf5d"].into(), 150000050000000000),
       (hex!["a0cc2a61879f21b7924392cfea5c35b47781f795ca24d179188c6d3f2a67952b"].into(), 60000050000000000),
       (hex!["2c6ce334da34c1ffdfb9cfb9962afdc9decf8f36b8d5282c2dbdef7c7b1aee53"].into(), 60000050000000000),
       (hex!["aa36b0d46767a839e11f18d8f15d373ed1f63abb33324edd87ebdc5fcfabd812"].into(), 60000050000000000),
       (hex!["ac6b20cfc19c17ca6d84edf5a082e242bdbb33c8f7f321e96f7764d3a9006d5a"].into(), 2812550000000000),
       (hex!["9a82629aac0895e5998542537f6b5b3a1c2c6fd46e827d409de88aacf9755a0e"].into(), 2812550000000000),
       (hex!["8039b9f35380bc3c20206d25c44006bd98e1252d7cb80acd6290b4f9c17bcd4c"].into(), 50000050000000000),
       (hex!["ec3cfd6b94a36adf49492caae5c59005b04e88a936c6106c4feca1631b5d6025"].into(), 50000050000000000),
       (hex!["8a442ebbcdb3aeace616292a957f36462e1e4c69e11de340527bfb617b01e068"].into(), 50000050000000000),
       (hex!["2c6789aa288e153564fe1ad4f824d8b760171db53d4e7500e2d3f9d51e979e03"].into(), 400000050000000000),
    ];
    let investor_vesting = vec![
       (hex!["e4cdc8abc0405db44c1a6886a2f2c59012fa3b98c07b61d63cc7f9e437ba243e"].into(), 1000, 28800, 3, 6000000000000000),
       (hex!["b26562a2e476fea86b26b2e47f12d279deb0ca7812bd1dad5b4fc8a909e10b22"].into(), 1000, 28800, 3, 80000000000000000),
       (hex!["e83adffb6338272e981cbc0c6cc03fd4e5e8447497b6b531b9436870c6079758"].into(), 1000, 28800, 3, 6000000000000000),
       (hex!["e613dd948e7baacc02c97db737ad43af7024f5ae595d06f1611ce827c300b17f"].into(), 1000, 28800, 3, 120000000000000000),
       (hex!["182400644f4780a65a43e00f9630152fe0ab2323d0dacd04e808ceccf462f416"].into(), 1000, 28800, 3, 35000000000000000),
       (hex!["b8779ddd7bc8dc00dc0e220b6b07b509553c3cdbdad3e384cc1ba2187cbca53f"].into(), 1000, 28800, 3, 1875000000000000),
       (hex!["62168680c9ed6e456fa59bd01525a53dd6fa991757e920482016e7db6caebd45"].into(), 1000, 28800, 3, 4375000000000000),
       (hex!["1c2eaec3bd844d93d29d442c1ecc431e8502ce7b13f950ae203ee63ff2a1750a"].into(), 1000, 28800, 3, 17500000000000000),
       (hex!["78d4caac9c5b562190901aafb9f2c74780c5831a89257254ca225729e755d919"].into(), 1000, 28800, 3, 17500000000000000),
       (hex!["1a8538e949213a4034bca131957bbfe8bc45107be4e93c86f92353fccff90039"].into(), 1000, 28800, 3, 17500000000000000),
       (hex!["fac6591fd5605154f1a77fc142d66d9c2f7b11f5c0bc61a3ac8ab46099e87e3a"].into(), 1000, 28800, 3, 5250000000000000),
       (hex!["ccb97ce4726461ad53c0ec9277b1ba3f7f88b0a11f847f1ca17d358e6e4d0a05"].into(), 1000, 28800, 3, 63750000000000000),
       (hex!["08a1c86a2c789eeb1295c3b3ba63b2cde5d23fa6c80d8f87246c21a11fa3ba1d"].into(), 1000, 28800, 3, 35000000000000000),
       (hex!["082cb53d6299dc033e467de007bfd5c4c0d24135aa85d2f1d983008ff78fbb66"].into(), 1000, 28800, 3, 42500000000000000),
       (hex!["48cb52f3831917977aec38d9c3a3c73c8253b82523af35d44b7122e674677f05"].into(), 1000, 28800, 3, 17500000000000000),
       (hex!["0617b168a08acd31e3323ff63cb6e8e7682ba002ca0184a59a0ebc6dcf4e7f2b"].into(), 1000, 28800, 3, 17500000000000000),
       (hex!["b2fa882baef6358e3b4379c290fc989093da5f62b0c8cc57bb972fa7232efe10"].into(), 1000, 28800, 3, 8750000000000000),
       (hex!["ecd0a0fba2f97d02d81fa3408e7e1f4a40b36d58fb7b999f0d0f5e073b810d3d"].into(), 1000, 28800, 3, 31875000000000000),
       (hex!["0838d06bad89b000120bea3e2cbf59e342f518a3f76becfa8c35bfd386e79825"].into(), 1000, 28800, 3, 17500000000000000),
       (hex!["60285b86e8196e4e20565440e2ded16459a8f1e8b6c5ce8bacb4a5b11eee8b05"].into(), 1000, 28800, 3, 25250000000000000),
       (hex!["68732830b518f410592bfb6f623e9864e9c021bc4adfe4845916932024bf9119"].into(), 1000, 28800, 3, 2625000000000000),
       (hex!["bc13c9a902a524609f064014695f2b6548a17d7e8bb12a834220559bc38bbc5d"].into(), 1000, 28800, 3, 14000000000000000),
       (hex!["daeb89c994d06f7e996e2c3e9e1fe685765e40f083432fbcdcb7f77bc1f9a378"].into(), 1000, 28800, 3, 2625000000000000),
       (hex!["3ceab1c17a4302ac0471e943279bd993adf12af6d2010a4f73bbdf428fba914f"].into(), 1000, 28800, 3, 10000000000000000),
       (hex!["baf1346f012c29003aeb63ac2503fbfafcd0dc182e98053b34f8bb08510ca73f"].into(), 1000, 28800, 3, 15280000000000000),
       (hex!["969554a9c50959bc434b99051b9803cc911ba3cad6c0e1d2ab2b8bcbbd1f057e"].into(), 1000, 28800, 3, 20000000000000000),
       (hex!["724513af8211cbaaeb17e7bbff8f2286718135d4ebe10e556c5b2076dbbd342d"].into(), 1000, 28800, 3, 20000000000000000),
       (hex!["eab1d6b0efce910517067712d026e42ab5f84ffd068b80d3cd55cd7c95d4db68"].into(), 1000, 28800, 3, 20000000000000000),
       (hex!["3ee90311650ce54b81d70f77537dc255c130ac9f5f5933cc6e2cedcb00ebdf5d"].into(), 1000, 28800, 3, 50000000000000000),
       (hex!["a0cc2a61879f21b7924392cfea5c35b47781f795ca24d179188c6d3f2a67952b"].into(), 1000, 28800, 3, 20000000000000000),
       (hex!["2c6ce334da34c1ffdfb9cfb9962afdc9decf8f36b8d5282c2dbdef7c7b1aee53"].into(), 1000, 28800, 3, 20000000000000000),
       (hex!["aa36b0d46767a839e11f18d8f15d373ed1f63abb33324edd87ebdc5fcfabd812"].into(), 1000, 28800, 3, 20000000000000000),
       (hex!["ac6b20cfc19c17ca6d84edf5a082e242bdbb33c8f7f321e96f7764d3a9006d5a"].into(), 1000, 28800, 3, 937500000000000),
       (hex!["9a82629aac0895e5998542537f6b5b3a1c2c6fd46e827d409de88aacf9755a0e"].into(), 1000, 28800, 3, 937500000000000),
       (hex!["8039b9f35380bc3c20206d25c44006bd98e1252d7cb80acd6290b4f9c17bcd4c"].into(), 1000, 28800, 1, 5000000000000000),
       (hex!["8039b9f35380bc3c20206d25c44006bd98e1252d7cb80acd6290b4f9c17bcd4c"].into(), 2650600, 28800, 4, 11250000000000000),
       (hex!["ec3cfd6b94a36adf49492caae5c59005b04e88a936c6106c4feca1631b5d6025"].into(), 1000, 28800, 1, 5000000000000000),
       (hex!["ec3cfd6b94a36adf49492caae5c59005b04e88a936c6106c4feca1631b5d6025"].into(), 2650600, 28800, 4, 11250000000000000),
       (hex!["8a442ebbcdb3aeace616292a957f36462e1e4c69e11de340527bfb617b01e068"].into(), 1000, 28800, 1, 5000000000000000),
       (hex!["8a442ebbcdb3aeace616292a957f36462e1e4c69e11de340527bfb617b01e068"].into(), 2650600, 28800, 4, 11250000000000000),
       (hex!["2c6789aa288e153564fe1ad4f824d8b760171db53d4e7500e2d3f9d51e979e03"].into(), 1000, 28800, 1, 40000000000000000),
       (hex!["2c6789aa288e153564fe1ad4f824d8b760171db53d4e7500e2d3f9d51e979e03"].into(), 2650600, 28800, 4, 90000000000000000),
    ];

    let mut endowed_accounts: Vec<AccountId> = endowed_accounts.unwrap_or_else(|| {
        vec![
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
            get_account_id_from_seed::<sr25519::Public>("Dave"),
            get_account_id_from_seed::<sr25519::Public>("Eve"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie"),
            get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
            get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
            get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
            get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
            get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
            get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
        ]
    });
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.0)
        .chain(initial_nominators.iter())
        .for_each(|x| {
            if !endowed_accounts.contains(&x) {
                endowed_accounts.push(x.clone())
            }
        });

    // stakers: all validators and nominators.
    let mut rng = rand::thread_rng();
    let stakers = initial_authorities
        .iter()
        .map(|x| (x.0.clone(), x.1.clone(), STASH, StakerStatus::Validator))
        .chain(initial_nominators.iter().map(|x| {
            use rand::{seq::SliceRandom, Rng};
            let limit = (MAX_NOMINATIONS as usize).min(initial_authorities.len());
            let count = rng.gen::<usize>() % limit;
            let nominations = initial_authorities
                .as_slice()
                .choose_multiple(&mut rng, count)
                .into_iter()
                .map(|choice| choice.0.clone())
                .collect::<Vec<_>>();
            (
                x.clone(),
                x.clone(),
                STASH,
                StakerStatus::Nominator(nominations),
            )
        }))
        .collect::<Vec<_>>();

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 10_000_000 * PDEX;
    const STASH: Balance = ENDOWMENT / 1000;
    let mut balances_vec:Vec<(AccountId,Balance)> = endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .collect();
                
    balances_vec.push((treasury_accont.clone(),100000000*PDEX));
    balances_vec.append(&mut investor_balances);
    GenesisConfig {
        frame_system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
            changes_trie_config: Default::default(),
        },
        pallet_balances: BalancesConfig {
            balances: balances_vec,
        },

        pallet_indices: IndicesConfig { indices: vec![] },
        pallet_session: SessionConfig {
            keys: initial_authorities
                .iter()
                .map(|x| {
                    (
                        x.0.clone(),
                        x.0.clone(),
                        session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone()),
                    )
                })
                .collect::<Vec<_>>(),
        },
        pallet_staking: StakingConfig {
            validator_count: initial_authorities.len() as u32 * 2,
            minimum_validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities.iter().map(|x| x.0.clone()).collect(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers,
            ..Default::default()
        },
        pallet_elections_phragmen: ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect(),
        },
        pallet_collective_Instance1: CouncilConfig::default(),
        pallet_collective_Instance2: TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect(),
            phantom: Default::default(),
        },
        pallet_contracts: ContractsConfig {
            // println should only be enabled on development chains
            current_schedule: pallet_contracts::Schedule::default().enable_println(enable_println),
        },
        pallet_sudo: SudoConfig {
            key: root_key.clone(),
        },
        pallet_babe: BabeConfig {
            authorities: vec![],
            epoch_config: Some(node_polkadex_runtime::BABE_GENESIS_EPOCH_CONFIG),
        },
        pallet_im_online: ImOnlineConfig { keys: vec![] },
        pallet_authority_discovery: AuthorityDiscoveryConfig { keys: vec![] },
        pallet_grandpa: GrandpaConfig {
            authorities: vec![],
        },
        pallet_membership_Instance1: Default::default(),
        pallet_treasury: Default::default(),
        pallet_vesting: Default::default(),
        orml_vesting: OrmlVestingConfig { vesting: investor_vesting },
        orml_tokens: Default::default(),
    }
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        true,
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
            //TODO should they still be here?
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        None,
        false,
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

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            vec![],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            None,
            false,
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
