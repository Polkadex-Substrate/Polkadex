"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u8 = void 0;

var _UInt = require("../codec/UInt");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name u8
 * @description
 * An 8-bit unsigned integer
 */
class u8 extends _UInt.UInt.with(8) {}

exports.u8 = u8;