"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.evmToAddress = evmToAddress;

var _util = require("@polkadot/util");

var _hasher = require("../secp256k1/hasher");

var _encode = require("./encode");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name evmToAddress
 * @summary Converts an EVM address to its corresponding SS58 address.
 */
function evmToAddress(evmAddress, ss58Format, hashType = 'blake2') {
  const wrapError = message => `Converting ${evmAddress}: ${message}`;

  const message = (0, _util.u8aConcat)('evm:', evmAddress);

  if (message.length !== 24) {
    throw new Error(wrapError('Invalid evm address length'));
  }

  const address = (0, _hasher.secp256k1Hasher)(hashType, message);
  return (0, _encode.encodeAddress)(address, ss58Format);
}