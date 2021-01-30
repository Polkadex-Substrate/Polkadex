"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelVrfVerify = schnorrkelVrfVerify;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const EMPTY_U8A = new Uint8Array();
/**
 * @name schnorrkelVrfVerify
 * @description Verify with sr25519 vrf verification
 */

function schnorrkelVrfVerify(message, signOutput, publicKey, context = EMPTY_U8A, extra = EMPTY_U8A) {
  const publicKeyU8a = (0, _util.u8aToU8a)(publicKey);
  const proofU8a = (0, _util.u8aToU8a)(signOutput);
  (0, _util.assert)(publicKeyU8a.length === 32, 'Invalid publicKey, expected 32-bytes');
  (0, _util.assert)(proofU8a.length === 96, 'Invalid vrfSign output, expected 96 bytes');
  return (0, _wasmCrypto.vrfVerify)(publicKeyU8a, (0, _util.u8aToU8a)(context), (0, _util.u8aToU8a)(message), (0, _util.u8aToU8a)(extra), proofU8a);
}