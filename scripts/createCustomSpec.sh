../target/release/substrate build-spec --disable-default-bootnode --chain local > customSpec.json
../target/release/substrate build-spec --chain=customSpec.json --raw --disable-default-bootnode > customSpecRaw.json
