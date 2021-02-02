"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isBase64 = isBase64;

var _validate = require("./validate");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function isBase64(value) {
  try {
    return (0, _validate.base64Validate)(value);
  } catch (error) {
    return false;
  }
}