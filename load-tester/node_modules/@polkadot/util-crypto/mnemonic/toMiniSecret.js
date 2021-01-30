"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.mnemonicToMiniSecret = mnemonicToMiniSecret;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _pbkdf = require("../pbkdf2");

var _toEntropy = require("./toEntropy");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function mnemonicToMiniSecret(mnemonic, password = '', onlyJs = false) {
  if ((0, _wasmCrypto.isReady)() && !onlyJs) {
    return (0, _wasmCrypto.bip39ToMiniSecret)(mnemonic, password);
  }

  const entropy = (0, _util.u8aToBuffer)((0, _toEntropy.mnemonicToEntropy)(mnemonic));
  const salt = (0, _util.u8aToBuffer)((0, _util.stringToU8a)(`mnemonic${password}`)); // return the first 32 bytes as the seed

  return (0, _pbkdf.pbkdf2Encode)(entropy, salt).password.slice(0, 32);
}