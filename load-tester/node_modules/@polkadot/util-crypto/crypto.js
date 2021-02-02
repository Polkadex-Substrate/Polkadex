"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.cryptoIsReady = cryptoIsReady;
exports.cryptoWaitReady = cryptoWaitReady;

var _wasmCrypto = require("@polkadot/wasm-crypto");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function cryptoIsReady() {
  return (0, _wasmCrypto.isReady)();
}

function cryptoWaitReady() {
  return (0, _wasmCrypto.waitReady)().then(() => true).catch(error => {
    console.error('Unable to initialize @polkadot/util-crypto', error);
    return false;
  });
}