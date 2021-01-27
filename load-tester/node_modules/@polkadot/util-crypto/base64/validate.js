"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base64Validate = base64Validate;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name base64Validate
 * @summary Validates a base64 value.
 * @description
 * Validates the the supplied value is valid base64
 */
function base64Validate(value) {
  (0, _util.assert)(value, 'Expected non-null, non-empty base64 input');
  (0, _util.assert)(/^(?:[A-Za-z0-9+/]{2}[A-Za-z0-9+/]{2})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value), 'Invalid base64 encoding');
  return true;
}