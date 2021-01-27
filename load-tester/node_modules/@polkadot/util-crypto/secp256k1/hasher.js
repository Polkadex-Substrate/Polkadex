"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.secp256k1Hasher = secp256k1Hasher;

var _blake = require("../blake2");

var _keccak = require("../keccak");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const HASH_TYPES = ['blake2', 'keccak'];

function secp256k1Hasher(hashType, data) {
  if (hashType === 'blake2') {
    return (0, _blake.blake2AsU8a)(data);
  } else if (hashType === 'keccak') {
    return (0, _keccak.keccakAsU8a)(data);
  }

  throw new Error(`Unsupported secp256k1 hasher '${hashType}', expected one of ${HASH_TYPES.join(', ')}`);
}