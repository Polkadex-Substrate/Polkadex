# Purge any chain data from previous runs
# You will be prompted to type `y`
./target/release/polkadex-node purge-chain --base-path /tmp/alice --chain local

# Start Alice's node
./target/release/polkadex-node \
  --base-path /tmp/alice \
  --chain local \
  --alice \
  --port 30333 \
  --ws-port 9945 \
  --rpc-port 9933 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url 'wss://telemetry.polkadot.io/submit/ 0' \
  --validator