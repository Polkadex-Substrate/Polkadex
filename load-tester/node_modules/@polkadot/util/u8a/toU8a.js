"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u8aToU8a = u8aToU8a;

var _toU8a = require("../buffer/toU8a");

var _toU8a2 = require("../hex/toU8a");

var _buffer = require("../is/buffer");

var _hex = require("../is/hex");

var _string = require("../is/string");

var _toU8a3 = require("../string/toU8a");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
function convertArray(value) {
  return Array.isArray(value) ? Uint8Array.from(value) : value;
}

function convertString(value) {
  return (0, _hex.isHex)(value) ? (0, _toU8a2.hexToU8a)(value) : (0, _toU8a3.stringToU8a)(value);
}
/**
 * @name u8aToU8a
 * @summary Creates a Uint8Array value from a Uint8Array, Buffer, string or hex input.
 * @description
 * `null` or `undefined` inputs returns a `[]` result, Uint8Array values returns the value, hex strings returns a Uint8Array representation.
 * @example
 * <BR>
 *
 * ```javascript
 * import { { u8aToU8a } from '@polkadot/util';
 *
 * u8aToU8a(new Uint8Array([0x12, 0x34]); // => Uint8Array([0x12, 0x34])
 * u8aToU8a(0x1234); // => Uint8Array([0x12, 0x34])
 * ```
 */


function u8aToU8a(value) {
  if (!value) {
    return new Uint8Array();
  } else if ((0, _buffer.isBuffer)(value)) {
    return (0, _toU8a.bufferToU8a)(value);
  } else if ((0, _string.isString)(value)) {
    return convertString(value);
  }

  return convertArray(value);
}