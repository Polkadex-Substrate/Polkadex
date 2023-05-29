# chainbridge-substrate

[![Build Status](https://travis-ci.com/ChainSafe/chainbridge-substrate.svg?branch=master)](https://travis-ci.com/ChainSafe/chainbridge-substrate)

Substrate implementation for [ChainBridge](https://github.com/ChainSafe/ChainBridge).

This repo contains two pallets:

## chainbridge

The core bridge logic. This handles voting and execution of proposals, administration of the relayer set and signaling
transfers.

## example-pallet

This pallet demonstrates how the chainbridge pallet can be integrated in to a substrate chain. It implements calls that
can be executed through proposal only and to initiate a basic transfer across the bridge.

## example-erc721

This pallet mimics an ERC721 token contract. It allows for minting, burning and transferring of tokens that consist of a
token ID (`U256`) and some metadata (`Vec<u8>`). This is also integrated into `example-pallet` to demonstrate how
non-fungibles can be transferred across the bridge.

# ChainSafe Security Policy

## Reporting a Security Bug

We take all security issues seriously, if you believe you have found a security issue within a ChainSafe
project please notify us immediately. If an issue is confirmed, we will take all necessary precautions
to ensure a statement and patch release is made in a timely manner.

Please email us a description of the flaw and any related information (e.g. reproduction steps, version) to
[security at chainsafe dot io](mailto:security@chainsafe.io).

LICENSE: GPL-3.0
