"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.numberToHex = numberToHex;

var _fixLength = require("../hex/fixLength");

var _null = require("../is/null");

var _undefined = require("../is/undefined");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name numberToHex
 * @summary Creates a hex value from a number.
 * @description
 * `null`/`undefined`/`NaN` inputs returns an empty `0x` result. `number` input values return the actual bytes value converted to a `hex`. With `bitLength` set, it converts the number to the equivalent size.
 * @example
 * <BR>
 *
 * ```javascript
 * import { numberToHex } from '@polkadot/util';
 *
 * numberToHex(0x1234); // => '0x1234'
 * numberToHex(0x1234, 32); // => 0x00001234
 * ```
 */
function numberToHex(value, bitLength = -1) {
  if ((0, _undefined.isUndefined)(value) || (0, _null.isNull)(value) || isNaN(value)) {
    return '0x';
  }

  return (0, _fixLength.hexFixLength)(value.toString(16), bitLength, true);
}