"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base64Trim = base64Trim;

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function base64Trim(value) {
  while (value.length && value[value.length - 1] === '=') {
    value = value.slice(0, -1);
  }

  return value;
}