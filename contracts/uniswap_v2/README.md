# Uniswap v2 Ink! contract

An ink! smart contract for polkapool which implements uniswap v2. The flow is inspired by Acala's dex pallet. https://github.com/AcalaNetwork/Acala/tree/master/modules/dex

## Building
It can be built using [cargo-contract](https://github.com/paritytech/cargo-contract) with the following command:
```
cargo +nightly-2021-06-21 contract build
```

## Integration testing with Redspot
```
yarn install
yarn test
```
## Testing

Use cargo to test:
```
cargo +nightly test
```