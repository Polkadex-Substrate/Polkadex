"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.xxhashAsHex = xxhashAsHex;

var _util = require("@polkadot/util");

var _asU8a = require("./asU8a");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name xxhashAsHex
 * @summary Creates a xxhash64 hex from the input.
 * @description
 * From either a `string`, `Uint8Array` or a `Buffer` input, create the xxhash64 and return the result as a hex string with the specified `bitLength`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { xxhashAsHex } from '@polkadot/util-crypto';
 *
 * xxhashAsHex('abc'); // => 0x44bc2cf5ad770999
 * ```
 */
function xxhashAsHex(data, bitLength = 64) {
  return (0, _util.u8aToHex)((0, _asU8a.xxhashAsU8a)(data, bitLength));
}