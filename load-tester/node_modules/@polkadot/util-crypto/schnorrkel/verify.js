"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelVerify = schnorrkelVerify;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name schnorrkelVerify
 * @description Verifies the signature of `message`, using the supplied pair
 */
function schnorrkelVerify(message, signature, publicKey) {
  const publicKeyU8a = (0, _util.u8aToU8a)(publicKey);
  const signatureU8a = (0, _util.u8aToU8a)(signature);
  (0, _util.assert)(publicKeyU8a.length === 32, `Invalid publicKey, received ${publicKeyU8a.length} bytes, expected 32`);
  (0, _util.assert)(signatureU8a.length === 64, `Invalid signature, received ${signatureU8a.length} bytes, expected 64`);
  return (0, _wasmCrypto.sr25519Verify)(signatureU8a, (0, _util.u8aToU8a)(message), publicKeyU8a);
}