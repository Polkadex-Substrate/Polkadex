"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.i256 = void 0;

var _Int = require("../codec/Int");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name i256
 * @description
 * A 256-bit signed integer
 */
class i256 extends _Int.Int.with(256) {}

exports.i256 = i256;