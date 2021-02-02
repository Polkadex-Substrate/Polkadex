"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u32 = void 0;

var _UInt = require("../codec/UInt");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name u32
 * @description
 * A 32-bit unsigned integer
 */
class u32 extends _UInt.UInt.with(32) {}

exports.u32 = u32;