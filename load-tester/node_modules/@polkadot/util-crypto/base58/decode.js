"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base58Decode = base58Decode;

var _util = require("@polkadot/util");

var _bs = require("./bs58");

var _validate = require("./validate");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name base58Decode
 * @summary Decodes a base58 value.
 * @description
 * From the provided input, decode the base58 and return the result as an `Uint8Array`.
 */
function base58Decode(value, ipfsCompat) {
  (0, _validate.base58Validate)(value, ipfsCompat);
  return (0, _util.bufferToU8a)(_bs.bs58.decode(value.substr(ipfsCompat ? 1 : 0)));
}