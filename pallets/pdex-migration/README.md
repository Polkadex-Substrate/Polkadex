## PDEX Migration pallet

### Importing a `pdex-migration` pallet

The first thing you need to do to add the `pdex-migration` pallet is to import the `pdex-migration` crate in your runtime's Cargo.toml file

`runtime/Cargo.toml`

```
[dependencies]
#--snip--
pallet-verifier-lightclient = { git = "https://github.com/Polkadex-Substrate/snowbridge", default-features = false }
pallet-eth-dispatch = { package = "artemis-dispatch", git = "https://github.com/Polkadex-Substrate/snowbridge", default-features = false }
pallet-basic-channel = { package = "artemis-basic-channel", git = "https://github.com/Polkadex-Substrate/snowbridge", default-features = false }
artemis-core = { git = "https://github.com/Polkadex-Substrate/snowbridge", default-features = false }

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

## Smart contract (How to deploy Polkadot-Ethereum Bridge smart contracts)

Repo: https://github.com/Polkadex-Substrate/snowbridge/tree/main/ethereum

### Development

Make sure you use a recent node version, e. g. with [nvm](https://github.com/nvm-sh/nvm#installing-and-updating):

```
nvm install 14.17.0
nvm use 14.17.0
```

Install dependencies with yarn:

```
yarn install
```

Create an `.env` file using the `env.template` as a template. Note that deploying to ropsten network requires setting the INFURA_PROJECT_ID and MNEMONIC environment variables.

### Testing

Run tests on the hardhat network:

```
npx hardhat test
```

### Deployment

Example: Deploy contracts to Ropsten network

```
npx hardhat deploy --network ropsten
...
reusing "ScaleCodec" at 0xc624558eFE2E31baCE1249E1B161A1f1dc58a6ab
reusing "BasicOutboundChannel" at 0x30E16792D89f1939dEFb60683A44E3917901C849
deploying "ERC20App" (tx: 0x8436d39495d4331072dd292438d7157d1db46648a289d99636444f92bc3c6de8)...: deployed at 0xe46B454A908cEd5A795FA7a2D106AcDdcf7ea45e with 1478775 gas
deploying "TestToken" (tx: 0x387e960e8085f124091f25c9364f81d1fd01b94ef93d1b192b039cb3b05dcd18)...: deployed at 0x0c7f69F66AB81BB257198BAa7f42fd1469E002a6 with 1316995 gas
```

It deploys `ScaleCodec`, `BasicOutboundChannel`, `ERC20App`, `TestToken` contracts to Ropsten network.

### Verify the contracts

We are using `@nomiclabs/hardhat-etherscan` to verify. The env variable `ETHERSCAN_API_KEY` is necessary to use this. You can get this key from etherscan.io

For ScaleCodec:

```
npx hardhat verify --network ropsten 0xc624558eFE2E31baCE1249E1B161A1f1dc58a6ab
```

For BasicOutboundChannel:

```
npx hardhat verify --network ropsten 0x30E16792D89f1939dEFb60683A44E3917901C849
```

For ERC20App:

```
npx hardhat verify --network ropsten 0xe46B454A908cEd5A795FA7a2D106AcDdcf7ea45e --constructor-args ./deploy/arguments.js
```

For TestToken:

```
npx hardhat verify --network ropsten 0x0c7f69F66AB81BB257198BAa7f42fd1469E002a6 "Test Token" "TEST"
```

### Demo Token Migration

#### Mint `TEST` token to your account

Go to [`TEST` token contract](https://ropsten.etherscan.io/address/0x0c7f69F66AB81BB257198BAa7f42fd1469E002a6#writeContract).
Call `mint` function with the following:

- <strong>to (address)</strong>: your account address
- <strong>\_amount (uint256)</strong>: amount of tokens to mint (1000 \* 10^8)

#### Approve `TEST` token (in your account) to be spent by `ERC20App` contract

Go to [`TEST` token contract](https://ropsten.etherscan.io/address/0x0c7f69F66AB81BB257198BAa7f42fd1469E002a6#writeContract).
Call `approve` function with the following:

- <strong>spender (address)</strong>: ERC20App contract address (e.g: 0xe46B454A908cEd5A795FA7a2D106AcDdcf7ea45e)
- <strong>\_amount (uint256)</strong>: amount of tokens to lock (1 \* 10^8)

#### Authorize `ERC20App` as an operator in `BasicOutboundChannel` contract

Go to [`BasicOutboundChannel` token contract](https://ropsten.etherscan.io/address/0x30E16792D89f1939dEFb60683A44E3917901C849#writeContract).
Call `authorizeOperator` function with the following:

- <strong>operator (address)</strong>: ERC20App contract address (e.g: 0xe46B454A908cEd5A795FA7a2D106AcDdcf7ea45e)

#### Lock `TEST` token in `ERC20App` contract that actually triggers to migration

Go to [`ERC20App` token contract](https://ropsten.etherscan.io/address/0xe46B454A908cEd5A795FA7a2D106AcDdcf7ea45e#writeContract).

Call `lock` function with the following:

- <strong>\_token (address)</strong>: TEST token address (e.g: 0x0c7f69F66AB81BB257198BAa7f42fd1469E002a6)
- <strong>\_recipient (byte32)</strong>: The public key of the Polkadex account address (e.g: 0xcc816e946438b2b21b8a3073f983ce03ee0feb313ec494e2dec462cfb4e77502)
- <strong>\_amount (uint256)</strong>: amount of token to lock (e.g: 1 \* 10^8)
- <strong>\_channelId (uint8)</strong>: 0 (representing OutboundChannel)
