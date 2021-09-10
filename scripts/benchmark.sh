#!/bin/bash

install -d benchout
for i in  frame_system pallet pallet_babe pallet_balances pallet_bounties pallet_collective pallet_elections_phragmen pallet_grandpa pallet_identity pallet_im_online pallet_indices pallet_membership pallet_multisig pallet_proxy pallet_scheduler pallet_session pallet_staking pallet_timestamp pallet_treasury pallet_utility
do
   echo Try $i
   ./target/release/polkadex-node benchmark   --chain soba  --pallet=$i --extrinsic=\*  --output=benchout/$i
done
