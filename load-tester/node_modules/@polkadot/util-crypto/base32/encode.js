"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base32Encode = base32Encode;

var _util = require("@polkadot/util");

var _bs = require("./bs32");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
// adapted from https://github.com/multiformats/js-multibase/blob/424709195b46ffb1d6f2f69a7707598ebe751e5e/src/rfc4648.js
const MASK = (1 << _bs.BITS_PER_CHAR) - 1;
/**
 * @name base32Encode
 * @summary Creates a base32 value.
 * @description
 * From the provided input, create the base32 and return the result as a string.
 */

function base32Encode(value, ipfsCompat = false) {
  const u8a = (0, _util.u8aToU8a)(value);
  let out = '';
  let bits = 0;
  let buffer = 0;

  for (let i = 0; i < u8a.length; ++i) {
    buffer = buffer << 8 | u8a[i];
    bits += 8;

    while (bits > _bs.BITS_PER_CHAR) {
      bits -= _bs.BITS_PER_CHAR;
      out += _bs.BASE32_ALPHABET[MASK & buffer >> bits];
    }
  }

  if (bits) {
    out += _bs.BASE32_ALPHABET[MASK & buffer << _bs.BITS_PER_CHAR - bits];
  }

  return ipfsCompat ? `b${out}` : out;
}