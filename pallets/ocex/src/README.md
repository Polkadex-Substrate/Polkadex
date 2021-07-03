# Polkadex OCEX Pallet

Polkadex Fungible Assets Pallet provides functionality to issue new tokens and offers efficient way to mint, burn and distribute tokens.

## Overview
Polkadex Fungible Assets Pallet provides function for:-
* Creating new tokens.
* Setting Vesting information related to given Asset Id.
* Claiming Vesting amount, set by given Asset Id's creator.
* Setting Metadata of given Asset Id.
* Minting, Burning and Attesting tokens.


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

- `deposit` -
- `release` -
- `withdraw` -
- `register` - Registers main Account
- `add_proxy` - Adds Proxy Account for given Main Account.
- `remove_proxy` - Removes Proxy Account for given Main Account.




