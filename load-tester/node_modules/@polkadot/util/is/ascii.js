"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isAscii = isAscii;

var _toU8a = require("../u8a/toU8a");

var _string = require("./string");

// Copyright 2017-2021 @polkadot/util authors & contributors
// SPDX-License-Identifier: Apache-2.0
const FORMAT = [9, 10, 13];
/**
 * @name isAscii
 * @summary Tests if the input is printable ASCII
 * @description
 * Checks to see if the input string or Uint8Array is printable ASCII, 32-127 + formatters
 */

function isAscii(value) {
  return value ? !(0, _toU8a.u8aToU8a)(value).some(byte => byte >= 127 || byte < 32 && !FORMAT.includes(byte)) : (0, _string.isString)(value);
}