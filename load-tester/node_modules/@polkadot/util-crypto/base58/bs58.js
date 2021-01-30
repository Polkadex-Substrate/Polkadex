"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bs58 = exports.BASE58_ALPHABET = void 0;

var _baseX = _interopRequireDefault(require("base-x"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
// https://github.com/cryptocoinjs/base-x#alphabets
const BASE58_ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
exports.BASE58_ALPHABET = BASE58_ALPHABET;
const bs58 = (0, _baseX.default)(BASE58_ALPHABET);
exports.bs58 = bs58;