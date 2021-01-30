"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclVerify = naclVerify;

var _tweetnacl = _interopRequireDefault(require("tweetnacl"));

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name naclSign
 * @summary Verifies the signature on the supplied message.
 * @description
 * Verifies the `signature` on `message` with the supplied `publicKey`. Returns `true` on sucess, `false` otherwise.
 * @example
 * <BR>
 *
 * ```javascript
 * import { naclVerify } from '@polkadot/util-crypto';
 *
 * naclVerify([...], [...], [...]); // => true/false
 * ```
 */
function naclVerify(message, signature, publicKey, onlyJs = false) {
  const messageU8a = (0, _util.u8aToU8a)(message);
  const publicKeyU8a = (0, _util.u8aToU8a)(publicKey);
  const signatureU8a = (0, _util.u8aToU8a)(signature);
  (0, _util.assert)(publicKeyU8a.length === 32, `Invalid publicKey, received ${publicKeyU8a.length}, expected 32`);
  (0, _util.assert)(signatureU8a.length === 64, `Invalid signature, received ${signatureU8a.length} bytes, expected 64`);
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.ed25519Verify)(signatureU8a, messageU8a, publicKeyU8a) : _tweetnacl.default.sign.detached.verify(messageU8a, signatureU8a, publicKeyU8a);
}