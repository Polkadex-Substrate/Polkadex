#!/usr/bin/env bash

export NIGHTLY=nightly-2021-06-21
echo $NIGHTLY

rustup toolchain add $NIGHTLY
rustup target add wasm32-unknown-unknown --toolchain  $NIGHTLY
rustup component add rust-src  --toolchain  $NIGHTLY
# rustup target add x86_64-unknown-linux-gnu  --toolchain  $NIGHTLY
# cargo +$NIGHTLY contract build
# cargo +$NIGHTLY contract build