"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base64Encode = base64Encode;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name base64Encode
 * @summary Creates a base64 value.
 * @description
 * From the provided input, create the base64 and return the result as a string.
 */
function base64Encode(value) {
  return (0, _util.u8aToBuffer)((0, _util.u8aToU8a)(value)).toString('base64');
}