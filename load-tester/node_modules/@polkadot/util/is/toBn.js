"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isToBn = isToBn;

var _function = require("./function");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
function isToBn(value) {
  return !!value && (0, _function.isFunction)(value.toBn);
}