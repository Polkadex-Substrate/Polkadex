"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.xxhashAsU8a = xxhashAsU8a;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _asBn = _interopRequireDefault(require("./xxhash64/asBn"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name xxhashAsU8a
 * @summary Creates a xxhash64 u8a from the input.
 * @description
 * From either a `string`, `Uint8Array` or a `Buffer` input, create the xxhash64 and return the result as a `Uint8Array` with the specified `bitLength`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { xxhashAsU8a } from '@polkadot/util-crypto';
 *
 * xxhashAsU8a('abc'); // => 0x44bc2cf5ad770999
 * ```
 */
function xxhashAsU8a(data, bitLength = 64, onlyJs = false) {
  const iterations = Math.ceil(bitLength / 64);

  if ((0, _wasmCrypto.isReady)() && !onlyJs) {
    return (0, _wasmCrypto.twox)((0, _util.u8aToU8a)(data), iterations);
  }

  const u8a = new Uint8Array(Math.ceil(bitLength / 8));

  for (let seed = 0; seed < iterations; seed++) {
    u8a.set((0, _asBn.default)(data, seed).toArray('le', 8), seed * 8);
  }

  return u8a;
}