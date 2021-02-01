../target/release/substrate purge-chain --base-path /tmp/alice --chain local
../target/release/substrate \
  --base-path /tmp/alice \
  --chain customSpecRaw.json \
  --alice \
  --port 30333 \
  --ws-port 9944 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --validator
