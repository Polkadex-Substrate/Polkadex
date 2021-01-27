"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.ethereumEncode = ethereumEncode;

var _util = require("@polkadot/util");

var _keccak = require("../keccak");

var _secp256k = require("../secp256k1");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function getH160(u8a) {
  if ([33, 65].includes(u8a.length)) {
    u8a = (0, _keccak.keccakAsU8a)((0, _secp256k.secp256k1Expand)(u8a));
  }

  return u8a.slice(-20);
}

function ethereumEncode(addressOrPublic) {
  if (!addressOrPublic) {
    return '0x';
  }

  const u8aAddress = (0, _util.u8aToU8a)(addressOrPublic);
  (0, _util.assert)([20, 32, 33, 65].includes(u8aAddress.length), 'Invalid address or publicKey passed');
  const address = (0, _util.u8aToHex)(getH160(u8aAddress), -1, false);
  const hash = (0, _util.u8aToHex)((0, _keccak.keccakAsU8a)(address), -1, false);
  let result = '';

  for (let index = 0; index < 40; index++) {
    result = `${result}${parseInt(hash[index], 16) > 7 ? address[index].toUpperCase() : address[index]}`;
  }

  return `0x${result}`;
}