"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.checkMaxMin = checkMaxMin;

var _bn = _interopRequireDefault(require("bn.js"));

var _assert = require("../assert");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
function checkMaxMin(type, items) {
  (0, _assert.assert)(items.length >= 1, 'Must provide one or more BN arguments');
  return items.reduce((acc, val) => _bn.default[type](acc, val), items[0]);
}