"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.compactFromU8a = compactFromU8a;

var _bn = _interopRequireDefault(require("bn.js"));

var _u8a = require("../u8a");

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name compactFromU8a
 * @description Retrievs the offset and encoded length from a compact-prefixed value
 * @example
 * <BR>
 *
 * ```javascript
 * import { compactFromU8a } from '@polkadot/util';
 *
 * const [offset, length] = compactFromU8a(new Uint8Array([254, 255, 3, 0]), 32));
 *
 * console.log('value offset=', offset, 'length=', length); // 4, 0xffff
 * ```
 */
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function compactFromU8a(_input, bitLength = _defaults.DEFAULT_BITLENGTH) {
  const input = (0, _u8a.u8aToU8a)(_input);
  const flag = input[0] & 0b11;

  if (flag === 0b00) {
    return [1, new _bn.default(input[0]).shrn(2)];
  } else if (flag === 0b01) {
    return [2, (0, _u8a.u8aToBn)(input.slice(0, 2), true).shrn(2)];
  } else if (flag === 0b10) {
    return [4, (0, _u8a.u8aToBn)(input.slice(0, 4), true).shrn(2)];
  }

  const length = new _bn.default(input[0]).shrn(2) // clear flag
  .addn(4) // add 4 for base length
  .toNumber();
  const offset = 1 + length;
  return [offset, (0, _u8a.u8aToBn)(input.subarray(1, offset), true)];
}