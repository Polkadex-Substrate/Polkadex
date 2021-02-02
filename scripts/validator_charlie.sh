../target/release/substrate purge-chain --base-path /tmp/bob --chain local
../target/release/substrate \
  --base-path /tmp/bob \
  --chain customSpecRaw.json \
  --charlie \
  --port 30334 \
  --ws-port 9945 \
  --rpc-port 9934 \
  --validator \
  --node-key 0000000000000000000000000000000000000000000000000000000000000003 \
<<<<<<< HEAD
  --bootnodes /ip4/18.191.229.114/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
=======
  --bootnodes /ip4/18.217.129.225/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp
>>>>>>> 3612f18154229ebaa80342cafd3ee7abbcb00e0d
