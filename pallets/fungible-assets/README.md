# Polkadex Fungible Assets Pallet

Polkadex Fungible Assets Pallet provides functionality to issue new tokens and offers efficient way to mint, burn and distribute tokens.

## Overview
Polkadex Fungible Assets Pallet provides function for:-
* Creating new tokens.
* Setting Vesting information related to given Asset Id.
* Claiming Vesting amount, set by given Asset Id's creator.
* Setting Metadata of given Asset Id.
* Minting, Burning and Attesting tokens.


## Importing a `polkadex-fungible-assets` pallet

The first thing you need to do to add the `polkadex-fungible-assets` pallet is to import the `polkadex-fungible-assets` crate in your runtime's Cargo.toml file

`runtime/Cargo.toml`

```
[dependencies]
#--snip--
polkadex-fungible-assets = { git = "https://github.com/Polkadex-Substrate/Polkadex", version = "0.1.0", default-features = false }
```

Add the following two lines to the runtime's std feature.

```
[features]
default = ['std']
std = [
    #--snip--
    "polkadex-fungible-assets/std",
    #--snip--
]
```

### Adding the `polkadex-fungible-assets` pallet

`runtime/src/lib.rs`

- Adding to the construct_runtime! Macro

```
    PolkadexFungibleAsset: polkadex_fungible_assets::{Pallet, Call, Storage, Event<T>}
```

- Runtime Configuration

```
impl polkadex_fungible_assets::Config for Runtime {
	type Event = Event;
	type TreasuryAccountId = TreasuryAccountId;
	type GovernanceOrigin = EnsureGovernance;
	type NativeCurrency = BasicCurrencyAdapter<Runtime, Balances, Amount, BlockNumber>;
}
```

## Dispatchable functions

- `create_token` - Creates new Token and stores information related to that.
- `set_vesting_info` - Set Vesting information related to given Asset Id.
- `claim_vesting` - Claim Vesting amount, set by given Asset Id's creator.
- `set_metadata_fungible` - Set Metadata of given Asset Id.
- `mint_fungible` - Mints amount for given Asset Id.
- `burn_fungible` - Burns amount for given Asset Id.
- `attest_token` - Verifies given Asset Id.
- `modify_token_deposit_amount` - Modifies token deposit amount.



