#!/usr/bin/env bash
# This script is meant to be run on Unix/Linux based systems
set -e

echo "*** Initializing WASM build environment"

if [ -z $CI_PROJECT_NAME ] ; then
   rustup update nightly
   rustup update stable
fi
rustup toolchain add nightly-2021-05-11
rustup target add wasm32-unknown-unknown --toolchain nightly
rustup target add wasm32-unknown-unknown --toolchain nightly-2021-05-11
rustup target add x86_64-unknown-linux-gnu --toolchain nightly-2021-05-11
