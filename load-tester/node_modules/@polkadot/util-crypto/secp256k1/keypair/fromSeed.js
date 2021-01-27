"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.secp256k1KeypairFromSeed = secp256k1KeypairFromSeed;

var _util = require("@polkadot/util");

var _secp256k = require("../secp256k1");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name secp256k1KeypairFromSeed
 * @description Returns a object containing a `publicKey` & `secretKey` generated from the supplied seed.
 */
function secp256k1KeypairFromSeed(seed) {
  (0, _util.assert)(seed.length === 32, 'Expected valid 32-byte private key as a seed');

  const key = _secp256k.secp256k1.keyFromPrivate(seed);

  return {
    publicKey: new Uint8Array(key.getPublic().encodeCompressed()),
    secretKey: (0, _util.bnToU8a)(key.getPrivate(), _secp256k.EXPAND_OPT)
  };
}