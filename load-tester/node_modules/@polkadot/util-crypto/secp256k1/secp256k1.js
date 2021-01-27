"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.EXPAND_OPT = exports.secp256k1 = void 0;

var _elliptic = _interopRequireDefault(require("elliptic"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const EC = _elliptic.default.ec;
const secp256k1 = new EC('secp256k1');
exports.secp256k1 = secp256k1;
const EXPAND_OPT = {
  bitLength: 256,
  isLe: false
};
exports.EXPAND_OPT = EXPAND_OPT;