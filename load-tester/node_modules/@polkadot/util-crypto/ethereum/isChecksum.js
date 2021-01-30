"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.isEthereumChecksum = isEthereumChecksum;

var _util = require("@polkadot/util");

var _keccak = require("../keccak");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
function isInvalidChar(char, byte) {
  return byte > 7 && char !== char.toUpperCase() || byte <= 7 && char !== char.toLowerCase();
}

function isEthereumChecksum(_address) {
  const address = _address.replace('0x', '');

  const hash = (0, _util.u8aToHex)((0, _keccak.keccakAsU8a)(address.toLowerCase()), -1, false);

  for (let index = 0; index < 40; index++) {
    if (isInvalidChar(address[index], parseInt(hash[index], 16))) {
      return false;
    }
  }

  return true;
}