"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.wasmBytes = void 0;

var _buffer = require("./buffer");

var _fflate = require("./fflate");

// Copyright 2019-2021 @polkadot/wasm-crypto-wasm authors & contributors
// SPDX-License-Identifier: Apache-2.0
const wasmBytes = (0, _fflate.unzlibSync)(_buffer.buffer, new Uint8Array(_buffer.sizeUncompressed));
exports.wasmBytes = wasmBytes;