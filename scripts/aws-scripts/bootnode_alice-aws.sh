rm -r -f /tmp/alice
/polkadex-aura-node/prometheus/prometheus --config.file /polkadex-aura-node/prometheus/prometheus.yml &
/polkadex-aura-node/target/release/node-template --base-path /tmp/alice --chain /polkadex-aura-node/scripts/customSpecRaw.json --alice --port 30333 --ws-port 9955 --ws-external --rpc-methods=unsafe --node-key 0000000000000000000000000000000000000000000000000000000000000001 --validator --execution Native --prometheus-external
