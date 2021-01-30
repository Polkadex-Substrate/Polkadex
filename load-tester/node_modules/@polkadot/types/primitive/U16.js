"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.u16 = void 0;

var _UInt = require("../codec/UInt");

// Copyright 2017-2021 @polkadot/types authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name u16
 * @description
 * A 16-bit unsigned integer
 */
class u16 extends _UInt.UInt.with(16) {}

exports.u16 = u16;