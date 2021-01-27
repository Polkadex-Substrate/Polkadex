"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.checkAddress = checkAddress;

var _decode = require("../base58/decode");

var _checksum = require("./checksum");

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name checkAddress
 * @summary Validates an ss58 address.
 * @description
 * From the provided input, validate that the address is a valid input.
 */
function checkAddress(address, prefix) {
  let decoded;

  try {
    decoded = (0, _decode.base58Decode)(address);
  } catch (error) {
    return [false, error.message];
  }

  if (decoded[0] !== prefix) {
    return [false, `Prefix mismatch, expected ${prefix}, found ${decoded[0]}`];
  } else if (!_defaults.defaults.allowedEncodedLengths.includes(decoded.length)) {
    return [false, 'Invalid decoded address length'];
  }

  const [isValid] = (0, _checksum.checkAddressChecksum)(decoded);
  return [isValid, isValid ? null : 'Invalid decoded address checksum'];
}