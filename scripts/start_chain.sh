#!/usr/bin/env bash
#
# This file is part of Polkadex.
#
# Copyright (c) 2023 Polkadex oü.
# SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program. If not, see <https://www.gnu.org/licenses/>.

set -e

start_boot_node() {
  echo "Starting boot node..."
  install -d ../ind_validators/validator1
  cd ../ind_validators/validator1
  ../../target/$TARGET/polkadex-node --validator --base-path ./bootnode -lthea=trace -lorderbook=trace --rpc-port=9943 --thea-dummy-mode --chain=../../scripts/customSpecRaw.json --node-key=1f64f01767da8258fcb986bd68d6dff93dfcd49d0fc753cea27cf37ce91c3684 >out_boot_node 2>&1 &
  BOOT_NODE_PID=$(echo $!)
  cd ../../scripts
}

start_validator_1() {
  echo "Starting validator 1..."
  install -d ../ind_validators/validator2
  cd ../ind_validators/validator2
  ../../target/$TARGET/polkadex-node --validator --port 30334 --base-path ./validator01  --thea-dummy-mode \
    -lthea=trace -lorderbook=trace --rpc-port=9944 --chain=../../scripts/customSpecRaw.json \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30335/p2p/12D3KooWCMKvu1tJKQBjDZ4hN1saTP6D58e4WkwLZwks5cPpxqY7 \
    --node-key=d353c4b01db05aa66ddeab9d85c2fa2252368dd4961606e5985ed1e8f40dbc50 >out_validator_1 2>&1 &
  VALIDATOR_1_PID=$(echo $!)
  cd ../../scripts
}

start_validator_2() {
  echo "Starting validator 2..."
  install -d ../ind_validators/validator3
  cd ../ind_validators/validator3
  ../../target/$TARGET/polkadex-node --validator --port 30335 --base-path ./validator02 -lthea=trace --thea-dummy-mode \
    --rpc-port=9945 --chain=../../scripts/customSpecRaw.json \
    --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWRozCnsH7zCYiNVpCRqgaoxukPdYxqaPQNs9rdDMDeN4t \
    --bootnodes /ip4/127.0.0.1/tcp/30334/p2p/12D3KooWEVBdwVmV1BeAdtqzhjANK31ibYmLQXxEoeai4fx7KhNh \
    --node-key=24f121a84149f784f9fe3f1e2fb04e8873191a510bc4b073a3a815d78a29cf2d >out_validator_2 2>&1 &
  VALIDATOR_2_PID=$(echo $!)
  cd ../../scripts
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

  echo "Setting keys..."
  ./set-keys.sh
  sleep $SLEEP

  kill_nodes

  start_boot_node
  sleep $SLEEP

  start_validator_1
  start_validator_2

  sleep $SLEEP
  start_full_node

  echo "Finish Starting FullNode"

  tail -f ../ind_validators/full_node/out_full_node
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