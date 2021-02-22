/polkadex/prometheus/prometheus --config.file /polkadex/prometheus/prometheus.yml &
/polkadex/target/release/node-polkadex --chain /polkadex/scripts/customSpecRaw.json --dave --port 30333 --ws-port 9955 --rpc-port 9956 --ws-external --node-key 0000000000000000000000000000000000000000000000000000000000000005 --bootnodes /dns/balice-dev.polkadex.intra/tcp/30333/p2p/$p2pkey --prometheus-external
