rm -r -f /tmp/bob/
#/polkadex-aura-node/prometheus/prometheus --config.file /polkadex-aura-node/prometheus/prometheus.yml &
/polkadex/target/release/node-polkadex --base-path /tmp/bob --chain /polkadex/scripts/customSpecRaw.json --charlie --validator --ws-port 9959 --port 30666 --node-key 0000000000000000000000000000000000000000000000000000000000000003 --execution Native --prometheus-external --bootnodes /dns/bootnode_alice/tcp/30444/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
