"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.base64Pad = base64Pad;

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function base64Pad(value) {
  return value.padEnd(value.length + value.length % 4, '=');
}