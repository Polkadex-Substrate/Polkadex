"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.createKeyMulti = createKeyMulti;

var _util = require("@polkadot/util");

var _asU8a = require("../blake2/asU8a");

var _decode = require("./decode");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const PREFIX = (0, _util.stringToU8a)('modlpy/utilisuba');

function createKeyMulti(who, threshold) {
  return (0, _asU8a.blake2AsU8a)((0, _util.u8aConcat)(PREFIX, (0, _util.compactToU8a)(who.length), ...(0, _util.u8aSorted)(who.map(who => (0, _decode.decodeAddress)(who))), (0, _util.bnToU8a)(threshold, {
    bitLength: 16,
    isLe: true
  })));
}