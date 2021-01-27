"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.sha512AsU8a = sha512AsU8a;

var _tweetnacl = _interopRequireDefault(require("tweetnacl"));

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name sha512AsU8a
 * @summary Creates sha-512 hash of the input.
 * @description
 * Returns a sha-512 `Uint8Array` from the supplied data.
 * @example
 * <BR>
 *
 * ```javascript
 * import { sha512AsU8a } from '@polkadot/util-crypto';
 *
 * sha512AsU8a(Uint8Array.from([...])); // => Uint8Array([...])
 * ```
 */
function sha512AsU8a(data, onlyJs = false) {
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.sha512)(data) : _tweetnacl.default.hash(data);
}