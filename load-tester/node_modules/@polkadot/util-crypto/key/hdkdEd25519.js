"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.keyHdkdEd25519 = keyHdkdEd25519;

var _util = require("@polkadot/util");

var _deriveHard = require("../nacl/deriveHard");

var _fromSeed = require("../nacl/keypair/fromSeed");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function keyHdkdEd25519(keypair, {
  chainCode,
  isHard
}) {
  (0, _util.assert)(isHard, 'A soft key was found in the path (and is unsupported)');
  return (0, _fromSeed.naclKeypairFromSeed)((0, _deriveHard.naclDeriveHard)(keypair.secretKey.subarray(0, 32), chainCode));
}