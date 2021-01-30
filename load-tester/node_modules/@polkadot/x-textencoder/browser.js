"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.TextEncoder = void 0;

var _xGlobal = require("@polkadot/x-global");

var _fallback = require("./fallback");

// Copyright 2017-2021 @polkadot/x-textencoder authors & contributors
// SPDX-License-Identifier: Apache-2.0
const TextEncoder = typeof _xGlobal.xglobal.TextEncoder === 'undefined' ? _fallback.TextEncoder : _xGlobal.xglobal.TextEncoder;
exports.TextEncoder = TextEncoder;