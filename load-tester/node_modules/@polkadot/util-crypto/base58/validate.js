"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.validateChars = validateChars;
exports.base58Validate = base58Validate;

var _util = require("@polkadot/util");

var _bs = require("./bs58");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const BASE_CONFIG = {
  alphabet: _bs.BASE58_ALPHABET,
  ipfsChar: 'z',
  type: 'base58'
};

function validateChars({
  alphabet,
  ipfsChar,
  type
}, value, ipfsCompat) {
  (0, _util.assert)(value, `Expected non-null, non-empty ${type} input`);
  (0, _util.assert)(!ipfsCompat || value[0] === ipfsChar, `Expected ${type} to start with '${ipfsChar}'`);

  for (let i = ipfsCompat ? 1 : 0; i < value.length; i++) {
    (0, _util.assert)(alphabet.includes(value[i]), `Invalid ${type} character "${value[i]}" (0x${value.charCodeAt(i).toString(16)}) at index ${i}`);
  }

  return true;
}
/**
 * @name base58Validate
 * @summary Validates a base58 value.
 * @description
 * Validates the the supplied value is valid base58
 */


function base58Validate(value, ipfsCompat) {
  return validateChars(BASE_CONFIG, value, ipfsCompat);
}