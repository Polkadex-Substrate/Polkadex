rm -r -f /tmp/alice
/polkadex/prometheus/prometheus --config.file /polkadex/prometheus/prometheus.yml &
/polkadex/target/release/node-polkadex --base-path /tmp/alice --chain /polkadex/scripts/customSpecRaw.json --alice --port 30333 --ws-port 9955 --ws-external --rpc-methods=unsafe --node-key 0000000000000000000000000000000000000000000000000000000000000001 --validator --execution Native --prometheus-external
