rm -r -f /tmp/bob/
/polkadex-aura-node/prometheus/prometheus --config.file /polkadex-aura-node/prometheus/prometheus.yml &
/polkadex-aura-node/target/release/node-template --base-path /tmp/bob --chain /polkadex-aura-node/scripts/customSpecRaw.json --charlie --validator --ws-port 9955 --port 30333 --node-key 0000000000000000000000000000000000000000000000000000000000000003 --execution Native --prometheus-external --bootnodes /dns/balice-dev.polkadex.intra/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
