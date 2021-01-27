"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.default = xxhash64AsRaw;

var _asValue = _interopRequireDefault(require("./asValue"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function xxhash64AsRaw(data, seed) {
  return (0, _asValue.default)(data, seed).toString(16);
}