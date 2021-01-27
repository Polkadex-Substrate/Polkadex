"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.mnemonicToEntropy = mnemonicToEntropy;

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _bip = require("./bip39");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function mnemonicToEntropy(mnemonic, onlyJs = false) {
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.bip39ToEntropy)(mnemonic) : (0, _bip.mnemonicToEntropy)(mnemonic);
}