"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.stringCamelCase = stringCamelCase;

var _camelcase = _interopRequireDefault(require("camelcase"));

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name stringCamelCase
 * @summary Convert a dash/dot/underscore/space separated string/String to camelCase
 */
// eslint-disable-next-line @typescript-eslint/ban-types
function stringCamelCase(value) {
  return (0, _camelcase.default)(value.toString());
}