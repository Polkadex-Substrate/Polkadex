"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base32Validate = base32Validate;

var _validate = require("../base58/validate");

var _bs = require("./bs32");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const BASE_CONFIG = {
  alphabet: _bs.BASE32_ALPHABET,
  ipfsChar: 'b',
  type: 'base32'
};
/**
 * @name base32Validate
 * @summary Validates a base32 value.
 * @description
 * Validates the the supplied value is valid base32
 */

function base32Validate(value, ipfsCompat) {
  return (0, _validate.validateChars)(BASE_CONFIG, value, ipfsCompat);
}