"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isU8a = isU8a;

var _instanceOf = require("./instanceOf");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name isU8a
 * @summary Tests for a `Uint8Array` object instance.
 * @description
 * Checks to see if the input object is an instance of `Uint8Array`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { isUint8Array } from '@polkadot/util';
 *
 * console.log('isU8a', isU8a([])); // => false
 * ```
 */
function isU8a(value) {
  return (0, _instanceOf.isInstanceOf)(value, Uint8Array);
}