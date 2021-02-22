rm -r -f /tmp/bob/
/polkadex/prometheus/prometheus --config.file /polkadex/prometheus/prometheus.yml &
/polkadex/target/release/node-template --base-path /tmp/bob --chain /polkadex/scripts/customSpecRaw.json --charlie --validator --ws-port 9955 --port 30333 --node-key 0000000000000000000000000000000000000000000000000000000000000003 --execution Native --prometheus-external --bootnodes /dns/balice-dev.polkadex.intra/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
