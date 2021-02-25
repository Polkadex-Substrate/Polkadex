rm -r -f /tmp/alice
#/polkadex-aura-node/prometheus/prometheus --config.file /polkadex-aura-node/prometheus/prometheus.yml &
/polkadex/target/release/node-polkadex --base-path /tmp/alice --chain /polkadex/scripts/customSpecRaw.json --alice --port 30444 --ws-port 9957 --ws-external --rpc-methods=unsafe --node-key 0000000000000000000000000000000000000000000000000000000000000001 --validator --execution Native --prometheus-external
