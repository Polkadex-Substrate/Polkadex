"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelSign = schnorrkelSign;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name schnorrkelSign
 * @description Returns message signature of `message`, using the supplied pair
 */
function schnorrkelSign(message, {
  publicKey,
  secretKey
}) {
  (0, _util.assert)((publicKey === null || publicKey === void 0 ? void 0 : publicKey.length) === 32, 'Expected a valid publicKey, 32-bytes');
  (0, _util.assert)((secretKey === null || secretKey === void 0 ? void 0 : secretKey.length) === 64, 'Expected a valid secretKey, 64-bytes');
  return (0, _wasmCrypto.sr25519Sign)(publicKey, secretKey, (0, _util.u8aToU8a)(message));
}