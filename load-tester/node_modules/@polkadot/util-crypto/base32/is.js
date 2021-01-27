"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.testValidator = testValidator;
exports.isBase32 = isBase32;

var _validate = require("./validate");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function testValidator(validate, value, ipfsCompat) {
  try {
    return validate(value, ipfsCompat);
  } catch (error) {
    return false;
  }
}

function isBase32(value, ipfsCompat) {
  return testValidator(_validate.base32Validate, value, ipfsCompat);
}