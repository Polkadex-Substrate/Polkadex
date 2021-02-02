"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.secp256k1Compress = secp256k1Compress;

var _util = require("@polkadot/util");

var _secp256k = require("./secp256k1");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function secp256k1Compress(publicKey) {
  (0, _util.assert)([33, 65].includes(publicKey.length), 'Invalid publicKey provided');
  return new Uint8Array(_secp256k.secp256k1.keyFromPublic(publicKey).getPublic().encodeCompressed());
}