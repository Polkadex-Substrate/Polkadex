"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.scryptToU8a = scryptToU8a;

var _util = require("@polkadot/util");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function scryptToU8a(salt, {
  N,
  p,
  r
}) {
  return (0, _util.u8aConcat)(salt, (0, _util.bnToU8a)(N, {
    bitLength: 32,
    isLe: true
  }), (0, _util.bnToU8a)(p, {
    bitLength: 32,
    isLe: true
  }), (0, _util.bnToU8a)(r, {
    bitLength: 32,
    isLe: true
  }));
}