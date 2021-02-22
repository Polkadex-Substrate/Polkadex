#!/bin/bash
if [ "$1" = "mainnet" ]; then
  cargo build --manifest-path bin/polkadex/Cargo.toml --features mainnet
else
  cargo build --manifest-path bin/polkadex/Cargo.toml --features testnet
fi