"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelDeriveSoft = schnorrkelDeriveSoft;

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _fromU8a = require("./keypair/fromU8a");

var _toU8a = require("./keypair/toU8a");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function schnorrkelDeriveSoft(keypair, chainCode) {
  return (0, _fromU8a.schnorrkelKeypairFromU8a)((0, _wasmCrypto.sr25519DeriveKeypairSoft)((0, _toU8a.schnorrkelKeypairToU8a)(keypair), chainCode));
}