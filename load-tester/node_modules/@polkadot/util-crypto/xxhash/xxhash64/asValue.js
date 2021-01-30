"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = xxhash64AsValue;

var _xxhashjs = _interopRequireDefault(require("xxhashjs"));

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function xxhash64AsValue(data, seed) {
  if ((0, _util.isBuffer)(data) || (0, _util.isString)(data)) {
    return _xxhashjs.default.h64(data, seed);
  }

  return _xxhashjs.default.h64((0, _util.u8aToBuffer)(data), seed);
}