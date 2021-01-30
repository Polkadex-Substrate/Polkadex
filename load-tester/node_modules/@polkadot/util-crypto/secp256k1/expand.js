"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.secp256k1Expand = secp256k1Expand;

var _util = require("@polkadot/util");

var _secp256k = require("./secp256k1");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function secp256k1Expand(publicKey) {
  (0, _util.assert)([33, 65].includes(publicKey.length), 'Invalid publicKey provided');

  const expanded = _secp256k.secp256k1.keyFromPublic(publicKey).getPublic();

  return (0, _util.u8aConcat)((0, _util.bnToU8a)(expanded.getX(), _secp256k.EXPAND_OPT), (0, _util.bnToU8a)(expanded.getY(), _secp256k.EXPAND_OPT));
}