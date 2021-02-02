"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelVrfSign = schnorrkelVrfSign;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const EMPTY_U8A = new Uint8Array();
/**
 * @name schnorrkelVrfSign
 * @description Sign with sr25519 vrf signing (deterministic)
 */

function schnorrkelVrfSign(message, {
  secretKey
}, context = EMPTY_U8A, extra = EMPTY_U8A) {
  (0, _util.assert)((secretKey === null || secretKey === void 0 ? void 0 : secretKey.length) === 64, 'Invalid secretKey, expected 64-bytes');
  return (0, _wasmCrypto.vrfSign)(secretKey, (0, _util.u8aToU8a)(context), (0, _util.u8aToU8a)(message), (0, _util.u8aToU8a)(extra));
}