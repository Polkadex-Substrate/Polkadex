rm -r -f /tmp/bob/
#/polkadex-aura-node/prometheus/prometheus --config.file /polkadex-aura-node/prometheus/prometheus.yml &
/polkadex/target/release/node-polkadex --base-path /tmp/bob --chain /polkadex/scripts/customSpecRaw.json --bob --port 30555 --ws-port 9958 --rpc-port 9934 --validator --node-key 0000000000000000000000000000000000000000000000000000000000000002 --execution Native --prometheus-external --bootnodes /dns/bootnode_alice/tcp/30444/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
