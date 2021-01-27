"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.scryptEncode = scryptEncode;

var _scryptsy = _interopRequireDefault(require("scryptsy"));

var _util = require("@polkadot/util");

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _asU8a = require("../random/asU8a");

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function scryptEncode(passphrase, salt = (0, _asU8a.randomAsU8a)(), params = _defaults.DEFAULT_PARAMS) {
  const password = (0, _wasmCrypto.isReady)() ? (0, _wasmCrypto.scrypt)((0, _util.u8aToU8a)(passphrase), salt, Math.log2(params.N), params.r, params.p) : (0, _util.bufferToU8a)((0, _scryptsy.default)((0, _util.u8aToBuffer)((0, _util.u8aToU8a)(passphrase)), (0, _util.u8aToBuffer)(salt), params.N, params.r, params.p, 64));
  return {
    params,
    password,
    salt
  };
}