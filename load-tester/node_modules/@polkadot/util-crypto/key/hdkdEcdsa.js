"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.keyHdkdEcdsa = keyHdkdEcdsa;

var _util = require("@polkadot/util");

var _deriveHard = require("../secp256k1/deriveHard");

var _fromSeed = require("../secp256k1/keypair/fromSeed");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function keyHdkdEcdsa(keypair, {
  chainCode,
  isHard
}) {
  (0, _util.assert)(isHard, 'A soft key was found in the path (and is unsupported)');
  return (0, _fromSeed.secp256k1KeypairFromSeed)((0, _deriveHard.secp256k1DeriveHard)(keypair.secretKey.subarray(0, 32), chainCode));
}