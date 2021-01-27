"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.schnorrkelDerivePublic = schnorrkelDerivePublic;

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function schnorrkelDerivePublic(publicKey, chainCode) {
  return (0, _wasmCrypto.sr25519DerivePublicSoft)(publicKey, chainCode);
}