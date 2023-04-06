#!/bin/bash

../target/release/polkadex-node build-spec --disable-default-bootnode --chain udon > customSpec.json
../target/release/polkadex-node build-spec --chain=customSpec.json --raw --disable-default-bootnode > customSpecRaw.json
