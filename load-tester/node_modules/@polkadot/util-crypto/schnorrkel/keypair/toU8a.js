"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelKeypairToU8a = schnorrkelKeypairToU8a;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function schnorrkelKeypairToU8a({
  publicKey,
  secretKey
}) {
  return (0, _util.u8aConcat)(secretKey, publicKey).slice();
}