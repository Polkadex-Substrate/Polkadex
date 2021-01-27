"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.hexToU8a = hexToU8a;

var _assert = require("../assert");

var _hex = require("../is/hex");

var _stripPrefix = require("./stripPrefix");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name hexToU8a
 * @summary Creates a Uint8Array object from a hex string.
 * @description
 * `null` inputs returns an empty `Uint8Array` result. Hex input values return the actual bytes value converted to a Uint8Array. Anything that is not a hex string (including the `0x` prefix) throws an error.
 * @example
 * <BR>
 *
 * ```javascript
 * import { hexToU8a } from '@polkadot/util';
 *
 * hexToU8a('0x80001f'); // Uint8Array([0x80, 0x00, 0x1f])
 * hexToU8a('0x80001f', 32); // Uint8Array([0x00, 0x80, 0x00, 0x1f])
 * ```
 */
function hexToU8a(_value, bitLength = -1) {
  if (!_value) {
    return new Uint8Array();
  }

  (0, _assert.assert)((0, _hex.isHex)(_value), `Expected hex value to convert, found '${_value}'`);
  const value = (0, _stripPrefix.hexStripPrefix)(_value);
  const valLength = value.length / 2;
  const bufLength = Math.ceil(bitLength === -1 ? valLength : bitLength / 8);
  const result = new Uint8Array(bufLength);
  const offset = Math.max(0, bufLength - valLength);

  for (let index = 0; index < bufLength; index++) {
    result[index + offset] = parseInt(value.substr(index * 2, 2), 16);
  }

  return result;
}