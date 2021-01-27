"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.blake2AsU8a = blake2AsU8a;

var _blakejs = _interopRequireDefault(require("blakejs"));

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name blake2AsU8a
 * @summary Creates a blake2b u8a from the input.
 * @description
 * From a `Uint8Array` input, create the blake2b and return the result as a u8a with the specified `bitLength`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { blake2AsU8a } from '@polkadot/util-crypto';
 *
 * blake2AsU8a('abc'); // => [0xba, 0x80, 0xa53, 0xf98, 0x1c, 0x4d, 0x0d]
 * ```
 */
function blake2AsU8a(data, bitLength = 256, key = null, onlyJs = false) {
  const byteLength = Math.ceil(bitLength / 8);
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.blake2b)((0, _util.u8aToU8a)(data), (0, _util.u8aToU8a)(key), byteLength) : _blakejs.default.blake2b((0, _util.u8aToU8a)(data), key, byteLength);
}