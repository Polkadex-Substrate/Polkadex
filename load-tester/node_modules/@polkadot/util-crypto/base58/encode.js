"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base58Encode = base58Encode;

var _util = require("@polkadot/util");

var _bs = require("./bs58");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name base58Encode
 * @summary Creates a base58 value.
 * @description
 * From the provided input, create the base58 and return the result as a string.
 */
function base58Encode(value, ipfsCompat) {
  const out = _bs.bs58.encode((0, _util.u8aToBuffer)((0, _util.u8aToU8a)(value)));

  return ipfsCompat ? `z${out}` : out;
}