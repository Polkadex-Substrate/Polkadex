"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.i64 = void 0;

var _Int = require("../codec/Int");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name i64
 * @description
 * A 64-bit signed integer
 */
class i64 extends _Int.Int.with(64) {}

exports.i64 = i64;