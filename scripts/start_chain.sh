#!/usr/bin/env bash

set -e

start_boot_node() {
  echo "Starting boot node 1..."
  install -d ../ind_validators/validator1
  cd ../ind_validators/validator1
  ../../target/$TARGET/polkadex-node --validator --base-path ./bootnode -lafg=trace --ws-port=9943 --rpc-port=9944 --in-peers 200 --out-peers 200 --chain=../../scripts/customSpecRaw.json --node-key=1f64f01767da8258fcb986bd68d6dff93dfcd49d0fc753cea27cf37ce91c3684 >out_boot_node 2>&1 &
  BOOT_NODE_PID=$(echo $!)
  cd ../../scripts
}

start_validator_1() {
  echo "Starting validator 2..."
  install -d ../ind_validators/validator2
  cd ../ind_validators/validator2
  ../../target/$TARGET/polkadex-node --validator --port 30334 --base-path ./validator01 \
    -lthea=trace --ws-port=19945 --rpc-port=9945 --chain=../../scripts/customSpecRaw.json --in-peers 200 --out-peers 200 \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30335/p2p/12D3KooWCMKvu1tJKQBjDZ4hN1saTP6D58e4WkwLZwks5cPpxqY7 \
    --node-key=d353c4b01db05aa66ddeab9d85c2fa2252368dd4961606e5985ed1e8f40dbc50 >out_validator_2 2>&1 &
  VALIDATOR_1_PID=$(echo $!)
  cd ../../scripts
}

start_validator_2() {
  echo "Starting validator 3..."
  install -d ../ind_validators/validator3
  cd ../ind_validators/validator3
  ../../target/$TARGET/polkadex-node --validator --port 30335 --base-path ./validator02 -lthea=trace  \
    --ws-port=19947 --rpc-port=9946 --chain=../../scripts/customSpecRaw.json --in-peers 200 --out-peers 200 \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWEVBdwVmV1BeAdtqzhjANK31ibYmLQXxEoeai4fx7KhNh \
    --node-key=24f121a84149f784f9fe3f1e2fb04e8873191a510bc4b073a3a815d78a29cf2d >out_validator_3 2>&1 &
  VALIDATOR_2_PID=$(echo $!)
  cd ../../scripts
}

start_others() {
  for id in {4..20}
  do
    echo "Starting validator $id..."
    install -d ../ind_validators/validator$id
    cd ../ind_validators/validator$id
    ../../target/$TARGET/polkadex-node --validator --port $((30335 + $id)) --base-path ./validator0$id -lthea=trace  \
      --ws-port=$((19947 + $id)) --rpc-port=$((9943 + $id)) --chain=../../scripts/customSpecRaw.json \
      --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
      --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWEVBdwVmV1BeAdtqzhjANK31ibYmLQXxEoeai4fx7KhNh >out_validator_$id 2>&1 &
    cd ../../scripts
#    sleep 20
  done
}

start_full_node() {
  echo "Starting fullnode..."
  install -d ../ind_validators/full_node
  cd ../ind_validators/full_node
  ../../target/$TARGET/polkadex-node  --chain=../../scripts/customSpecRaw.json --pruning=archive \
  --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
  --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWEVBdwVmV1BeAdtqzhjANK31ibYmLQXxEoeai4fx7KhNh >out_full_node 2>&1 &
  FULLNODE_PID=$(echo $!)
  cd ../../scripts
}

kill_nodes() {
  echo "Killing nodes..."
  killall -2 polkadex-node
}

start_chain() {
  ./purge-chain.sh
  ./build-spec.sh

  start_boot_node
  sleep $SLEEP

  start_validator_1
  start_validator_2
  sleep $SLEEP
  start_others
  sleep 60

  echo "Setting keys..."
  ./set-keys.sh
  sleep $SLEEP

  kill_nodes
  sleep 30

  start_boot_node
  sleep $SLEEP

  start_validator_1
  start_validator_2
  sleep $SLEEP
  start_others

  tail -f ../ind_validators/validator3/out_validator_3
}

print_usage() {
  echo "usage: $0 [-dr] [-s time_in_secs]"
  echo "  -d|--debug                 runs debug target"
  echo "  -r|--release               runs release target (default)"
  echo "  -s|--sleep <time_in_secs>  seconds to wait between node starts, 3 secs by default"
  exit 1
}

# Default values
TARGET=release
SLEEP=10

parse_args() {
  while [[ $# -gt 0 ]]; do
    key="$1"

    case $key in
    -d | --debug)
      TARGET=debug
      shift # past value
      ;;
    -r | --release)
      TARGET=release
      shift # past value
      ;;
    -s | --sleep)
      SLEEP="$2"
      shift # past argument
      shift # past value
      ;;
    -h | --help)
      shift # past value
      print_usage
      ;;
    *) # unknown option
      echo "Unknown argument: $1"
      print_usage
      shift # past argument
      ;;
    esac
  done
}

parse_args $@

trap kill_nodes EXIT

start_chain
