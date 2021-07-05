## PDEX Migration pallet

### Importing a `pdex-migration` pallet

The first thing you need to do to add the `pdex-migration` pallet is to import the `pdex-migration` crate in your runtime's Cargo.toml file

`runtime/Cargo.toml`

```
[dependencies]
#--snip--
pallet-verifier-lightclient = { git = "https://github.com/Polkadex-Substrate/polkadot-ethereum", default-features = false, rev = "6ac2106a608bf11c53981b67a106c2afd43bbee6" }
pallet-eth-dispatch = { package = "artemis-dispatch", git = "https://github.com/Polkadex-Substrate/polkadot-ethereum", default-features = false, rev = "6ac2106a608bf11c53981b67a106c2afd43bbee6" }
pallet-basic-channel = { package = "artemis-basic-channel", git = "https://github.com/Polkadex-Substrate/polkadot-ethereum", default-features = false, rev = "6ac2106a608bf11c53981b67a106c2afd43bbee6" }
artemis-core = { git = "https://github.com/Polkadex-Substrate/polkadot-ethereum", default-features = false, rev = "6ac2106a608bf11c53981b67a106c2afd43bbee6" }

erc20-pdex-migration-pallet = { path = "https://github.com/Polkadex-Substrate/Polkadex/pallets/pdex-migration", default-features = false }
```

Add the following two lines to the runtime's std feature.

```
[features]
default = ['std']
std = [
    #--snip--
    'pallet-verifier-lightclient/std',
    'pallet-eth-dispatch/std',
    'pallet-basic-channel/std',
    'artemis-core/std',
    'erc20-pdex-migration-pallet/std',
    #--snip--
]
```

### Adding the `pdex-migration` pallet

`runtime/src/lib.rs`

- Adding to the construct_runtime! Macro

```
    Currencies: orml_currencies::{Pallet, Call, Event<T>},
    Dispatch: pallet_eth_dispatch::{Pallet, Call, Storage, Event<T>, Origin},
    ERC20PDEX: erc20_pdex_migration_pallet::{Pallet, Call, Storage, Config, Event<T>}
```

- Runtime Configuration

```
pub use artemis_core::MessageId;
use pallet_eth_dispatch::EnsureEthereumAccount;
pub use polkadex_primitives::{Balance};

pub struct CallFilter;

impl Filter<Call> for CallFilter {
    fn filter(call: &Call) -> bool {
        match call {
            Call::ERC20PDEX(_) => true,
            _ => false
        }
    }
}

impl pallet_eth_dispatch::Config for Runtime {
    type Origin = Origin;
    type Event = Event;
    type MessageId = MessageId;
    type Call = Call;
    type CallFilter = CallFilter;
}

impl erc20_pdex_migration_pallet::Config for Runtime{
    type Event = Event;
    type Balance = Balance;
    type Currency = Currencies;
    type CallOrigin = EnsureEthereumAccount;
}
```

### Genesis Configuration

- Generate EthereumHeader using `eth-relayer` (https://github.com/Polkadex-Substrate/eth-relayer)

```sh
./build/polkadex-eth-relay getblock
INFO[0002] Connected to chain                            chainID=3 endpoint="wss://ropsten.infura.io/ws/v3/"

EthereumHeader {
        parent_hash: hex!("30fe8b0a52f72f1e38556a0a32c3799e285855bcee53048935981028fcc49d44").into(),
        ...
}
```

- Update `initial_difficulty` for the certain network

`node/src/chain_spec.rs`
Inside the testnet_genesis function we need to add our pallet's configuration to the returned GenesisConfig object as followed:
```
    GenesisConfig {
        /* --snip-- */

        /*** Add This Block ***/
        // This is Ropsten Config
        pallet_verifier_lightclient: VerifierLightclientConfig {
            initial_header: EthereumHeader {
                parent_hash: hex!("c75694f43b710d53e3026151ecd910b4d1614ff6be90bea0e9e25c71d31ddc94").into(),
                timestamp: 1624172254u64.into(),
                number: 10473724u64.into(),
                author: hex!("1cffe205e97976bb9d1ec006f5222360a89353e0").into(),
                transactions_root: hex!("9e298e62573bb9fb4d774f48aacfef0299b5b2c711708e4c0966eaa3a297d507").into(),
                ommers_hash: hex!("1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347").into(),
                extra_data: hex!("d683010a02846765746886676f312e3136856c696e7578").into(),
                state_root: hex!("a7c3b4cda608f234e081945534f2c921ea82ee50ec1e5ec3117edf80e7e1d24b").into(),
                receipts_root: hex!("a6b13691149babc76431bc992102b658e2992ce48923acd830f189ee95838d2a").into(),
                logs_bloom: (&hex!("400202000000000000100000800000000000000000200000000000004000000a000400000100011000000040000000002000000000000800000000000420000000080000020100000000000800000002000004000200000000000000000020000400000802040000000000880000280000008000000000000000003000000000200000020002000020000a000100000040008400000080000000000200000200320000100000000020004004220000000000000000000000020000000000000100000002000000000000a00000000000000001000100000000000400000020000012000020100000000000000000002000000080000000000000001000010010")).into(),
                gas_used: 975581u64.into(),
                gas_limit: 8000000u64.into(),
                difficulty: 526959644u64.into(),
                seal: vec![
                    hex!("a0ef29b20dc8f835f811fd431be5af023ca83b9ab403838404ad09a86e4e27a52f").to_vec(),
                    hex!("88124e2d4f2bc1eda9").to_vec(),
                ],
            },
            initial_difficulty: 19755084633726428633088u128.into(),
        },
        basic_channel_inbound: BasicInboundChannelConfig {
            source_channel: hex!["EE9170ABFbf9421Ad6DD07F6BDec9D89F2B581E0"].into(),
        },
        erc20_pdex_migration_pallet: ERC20PDEXConfig {
            address: hex!["3f0839385DB9cBEa8E73AdA6fa0CFe07E321F61d"].into()
        },
        /*** End Added Block ***/
    }
```

### Dispatchable functions

- `mint()`

First, it checks if the function is called by `pallet_eth_dispatch` pallet. If not, it throws the `DispatchError` error.
Then, it mints `amount` of `AssetId:POLKADEX` tokens to the given `recipient` Polkadex address.
Finally, it emits `NativePDEXMinted` event with the various parameters.