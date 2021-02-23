rm -r -f /tmp/bob/
../target/release/node-polkadex --base-path /tmp/bob --chain customSpecRaw.json --bob --port 30333 --ws-port 9955 --ws-external --rpc-port 9934 --rpc-methods=unsafe --validator --node-key 0000000000000000000000000000000000000000000000000000000000000002 --execution Native
