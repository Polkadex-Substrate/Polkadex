"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.compactToU8a = compactToU8a;

var _bn = _interopRequireDefault(require("bn.js"));

var _assert = require("../assert");

var _bn2 = require("../bn");

var _u8a = require("../u8a");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
const MAX_U8 = new _bn.default(2).pow(new _bn.default(8 - 2)).subn(1);
const MAX_U16 = new _bn.default(2).pow(new _bn.default(16 - 2)).subn(1);
const MAX_U32 = new _bn.default(2).pow(new _bn.default(32 - 2)).subn(1);
/**
 * @name compactToU8a
 * @description Encodes a number into a compact representation
 * @example
 * <BR>
 *
 * ```javascript
 * import { compactToU8a } from '@polkadot/util';
 *
 * console.log(compactToU8a(511, 32)); // Uint8Array([0b11111101, 0b00000111])
 * ```
 */

function compactToU8a(_value) {
  const value = (0, _bn2.bnToBn)(_value);

  if (value.lte(MAX_U8)) {
    return new Uint8Array([value.toNumber() << 2]);
  } else if (value.lte(MAX_U16)) {
    return (0, _bn2.bnToU8a)(value.shln(2).addn(0b01), 16, true);
  } else if (value.lte(MAX_U32)) {
    return (0, _bn2.bnToU8a)(value.shln(2).addn(0b10), 32, true);
  }

  const u8a = (0, _bn2.bnToU8a)(value);
  let length = u8a.length; // adjust to the minimum number of bytes

  while (u8a[length - 1] === 0) {
    length--;
  }

  (0, _assert.assert)(length >= 4, 'Previous tests match anyting less than 2^30; qed');
  return (0, _u8a.u8aConcat)(new Uint8Array([// substract 4 as minimum (also catered for in decoding)
  (length - 4 << 2) + 0b11]), u8a.subarray(0, length));
}