../target/release/substrate purge-chain --base-path /tmp/bob --chain local
../target/release/substrate \
  --base-path /tmp/bob \
  --chain customSpecRaw.json \
  --bob \
  --port 30334 \
  --ws-port 9945 \
  --rpc-port 9934 \
  --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000002 \
  --bootnodes /ip4/3.15.171.128/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
