# Polkadex OCEX Pallet

OCEX pallet handles the deposits, withdrawals and off-chain workers for verifying the IPFS snapshots of enclave, and safeguards user funds by initiating emergency shutdown protocol.

## Importing a `polkadex-ocex` pallet

The first thing you need to do to add the `polkadex-ocex` pallet is to import the `polkadex-ocex` crate in your runtime's Cargo.toml file

`runtime/Cargo.toml`

```
[dependencies]
#--snip--
polkadex-ocex = { git = "https://github.com/Polkadex-Substrate/Polkadex", default-features = false }
```

Add the following two lines to the runtime's std feature.

```
[features]
default = ['std']
std = [
    #--snip--
    "polkadex-ocex/std",
    #--snip--
]
```

### Adding the `polkadex-ocex` pallet

`runtime/src/lib.rs`

- Adding to the construct_runtime! Macro

```
    PolkadexOcex: polkadex_ocex::{Pallet, Call, Storage, Config<T>, Event<T>}
```

- Runtime Configuration

```
parameter_types! {
    pub const ProxyLimit: usize = 10; // Max sub-accounts per main account
    pub const OcexModuleId: PalletId = PalletId(*b"polka/ex");
    pub const OCEXGenesisAccount: PalletId = PalletId(*b"polka/ga");
}

impl polkadex_ocex::Config for Runtime {
	type Event = Event;
	type OcexId = OcexModuleId;
	type GenesisAccount = OCEXGenesisAccount;
	type Currency = Currencies;
	type ProxyLimit = ProxyLimit;
}
```
### Genesis Configuration
`node/src/chain_spec.rs`
Inside the testnet_genesis function we need to add our pallet's configuration to the returned GenesisConfig object as followed:
```
    GenesisConfig {
        /* --snip-- */
        /*** Add This Block ***/
        polkadex_ocex: PolkadexOcexConfig {
            key: genesis.clone(),
            genesis_account: genesis,
        }
        /*** End Added Block ***/
    }
```
## Dispatchable functions

- `deposit` - Transfers given amount to Enclave.
- `release` - Releases/Transfers given amount to Destination Account, Only Enclave can call this Dispatchable function.
- `withdraw` - Notifies enclave about senders intend to withdraw via on-chain.
- `register` - Registers main Account
- `add_proxy` - Adds Proxy Account for given Main Account.
- `remove_proxy` - Removes Proxy Account for given Main Account.




