"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclSign = naclSign;

var _tweetnacl = _interopRequireDefault(require("tweetnacl"));

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name naclSign
 * @summary Signs a message using the supplied secretKey
 * @description
 * Returns message signature of `message`, using the `secretKey`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { naclSign } from '@polkadot/util-crypto';
 *
 * naclSign([...], [...]); // => [...]
 * ```
 */
function naclSign(message, {
  publicKey,
  secretKey
}, onlyJs = false) {
  (0, _util.assert)(secretKey, 'Expected a valid secretKey');
  const messageU8a = (0, _util.u8aToU8a)(message);
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.ed25519Sign)(publicKey, secretKey.subarray(0, 32), messageU8a) : _tweetnacl.default.sign.detached(messageU8a, secretKey);
}