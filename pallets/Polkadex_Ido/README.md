# Polkadex IDO Pallet

Polkadex IDO Pallet provides functionality for whitelisting, distributing tokens and vesting done fully on the blockchain. It provdies one-stop-shop efficient solution for all the activities from conducting the fundraise to listing tokens on Polkadex exchange fully on-chain. It makes it easier to deploy tokens and distribute them among IDO participants. Teams will receive an end-to-end product that allows them to create tokens, bridge to other networks, list in the Orderbook, conduct IDOs and also finally migrate to their own blockchains later on in the project’s development.

## Overview
Polkadex IDO Pallet provides function for:-
* Registering a new investor to allow participating in funding round.
* Attest the investor to take part in the IDO pallet.
* Registration of funding round with the total allocation and Vesting period.
* Whitelisting investor.
* Storing information about whitelisted investor.
* Claiming token.

## Importing a `polkadex-ido` pallet

The first thing you need to do to add the `polkadex-ido` pallet is to import the `polkadex-ido` crate in your runtime's Cargo.toml file

`runtime/Cargo.toml`

```
[dependencies]
#--snip--
polkadex-ido = { git = "https://github.com/Polkadex-Substrate/Polkadex", default-features = false }
```

Add the following two lines to the runtime's std feature.

```
[features]
default = ['std']
std = [
    #--snip--
    'polkadex-ido/std',
    #--snip--
]
```

### Adding the `polkadex-ido` pallet

`runtime/src/lib.rs`

- Adding to the construct_runtime! Macro

```
    PolkadexIdo: polkadex_ido::{Pallet, Call, Event<T>}
```

- Runtime Configuration

```
parameter_types! {
    pub const GetIDOPDXAmount: Balance = 100u128;
    pub const GetMaxSupply: Balance = 2_000_000_0u128;
    pub const PolkadexIdoPalletId: PalletId = PalletId(*b"polk/ido");
}

impl polkadex_ido::Config for Runtime {
	type Event = Event;
	type TreasuryAccountId = TreasuryAccountId;
	type GovernanceOrigin = EnsureGovernance;
	type NativeCurrencyId = GetNativeCurrencyId;
	type IDOPDXAmount = GetIDOPDXAmount;
	type MaxSupply = GetMaxSupply;
	type Randomness = RandomnessCollectiveFlip;
	type ModuleId = PolkadexIdoPalletId;
	type Currency = Currencies;
	type WeightIDOInfo = polkadex_ido::weights::SubstrateWeight<Runtime>;

}
```

## Dispatchable functions

- `register_investor` - Registers a new investor to allow participating in funding round.
- `attest_investor` - Attests the investor to take part in the IDO pallet.
- `register_round` - Registers a funding round with the amount as the total allocation for this round and vesting period.
- `whitelist_investor` - Whitelists investor for the given round for the given amount.
- `participate_in_round` - Stores information about whitelisted investor, participating in round.
- `claim_tokens` - Claims token allocated for Investors.
- `show_interest_in_round` - Stores information about investors, showing interest in funding round.
- `withdraw_raise` - Transfers the raised amount to another address, only the round creator can call this or the governance.
- `withdraw_token` - Transfers the remaining tokens to another address, only the round creator can call this or the governance.


