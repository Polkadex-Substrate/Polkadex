#!/bin/bash -e
cargo build --release --features runtime-benchmarks
./target/release/polkadex-node benchmark --chain dev --list


install -d benchout
#for i in frame_system pallet_babe pallet_balances pallet_bounties pallet_collective pallet_elections_phragmen pallet_grandpa pallet_identity pallet_im_online pallet_indices pallet_membership pallet_multisig pallet_proxy pallet_scheduler pallet_session pallet_staking pallet_timestamp pallet_treasury pallet_utility; do
for i in `./target/release/polkadex-node benchmark --chain dev --list | sed s/,.*// |sort |uniq` ; do
   echo Try $i
   echo   ./target/release/polkadex-node benchmark \
      --chain dev \
      --execution wasm \
      --wasm-execution compiled \
      --pallet=$i \
      --extrinsic="*" \
      --steps 20 \
      --repeat 50 \
      --output=benchout/$i.rs
done
