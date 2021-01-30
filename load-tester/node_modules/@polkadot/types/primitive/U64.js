"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u64 = void 0;

var _UInt = require("../codec/UInt");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name u64
 * @description
 * A 64-bit unsigned integer
 */
class u64 extends _UInt.UInt.with(64) {}

exports.u64 = u64;