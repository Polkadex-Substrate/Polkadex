../prometheus/prometheus --config.file ../prometheus/prometheus.yml &
../target/release/node-polkadex --chain customSpecRaw.json --dave --port 30333 --ws-port 9955 --rpc-port 9956 --node-key 0000000000000000000000000000000000000000000000000000000000000005 --bootnodes /ip4/54.176.87.85/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp --prometheus-external
