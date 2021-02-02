"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.pbkdf2Encode = pbkdf2Encode;

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _asU8a = require("../random/asU8a");

var _pbkdf = require("./pbkdf2");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function pbkdf2Encode(passphrase, salt = (0, _asU8a.randomAsU8a)(), rounds = 2048, onlyJs = false) {
  const u8aPass = (0, _util.u8aToU8a)(passphrase);
  const u8aSalt = (0, _util.u8aToU8a)(salt);
  const password = (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.pbkdf2)(u8aPass, u8aSalt, rounds) : (0, _pbkdf.pbkdf2Sync)(u8aPass, u8aSalt, rounds);
  return {
    password,
    rounds,
    salt
  };
}