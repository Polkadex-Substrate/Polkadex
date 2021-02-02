"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isHex = isHex;

var _string = require("./string");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
const HEX_REGEX = /^0x[a-fA-F0-9]+$/;
/**
 * @name isHex
 * @summary Tests for a hex string.
 * @description
 * Checks to see if the input value is a `0x` prefixed hex string. Optionally (`bitLength` !== -1) checks to see if the bitLength is correct.
 * @example
 * <BR>
 *
 * ```javascript
 * import { isHex } from '@polkadot/util';
 *
 * isHex('0x1234'); // => true
 * isHex('0x1234', 8); // => false
 * ```
 */
// eslint-disable-next-line @typescript-eslint/ban-types

function isHex(value, bitLength = -1, ignoreLength = false) {
  const isValidHex = value === '0x' || (0, _string.isString)(value) && HEX_REGEX.test(value.toString());

  if (isValidHex && bitLength !== -1) {
    return value.length === 2 + Math.ceil(bitLength / 4);
  }

  return isValidHex && (ignoreLength || value.length % 2 === 0);
}