"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclDeriveHard = naclDeriveHard;

var _util = require("@polkadot/util");

var _asU8a = require("../blake2/asU8a");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const HDKD = (0, _util.compactAddLength)((0, _util.stringToU8a)('Ed25519HDKD'));

function naclDeriveHard(seed, chainCode) {
  return (0, _asU8a.blake2AsU8a)((0, _util.u8aConcat)(HDKD, seed, chainCode));
}