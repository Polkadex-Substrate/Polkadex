"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.secp256k1Verify = secp256k1Verify;

var _util = require("@polkadot/util");

var _expand = require("./expand");

var _hasher = require("./hasher");

var _secp256k = require("./secp256k1");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name secp256k1Verify
 * @description Verifies the signature of `message`, using the supplied pair
 */
function secp256k1Verify(message, signature, address, hashType = 'blake2') {
  const isEthereum = hashType === 'keccak';
  const u8a = (0, _util.u8aToU8a)(signature);
  (0, _util.assert)(u8a.length === 65, `Expected signature with 65 bytes, ${u8a.length} found instead`);
  const publicKey = new Uint8Array( // eslint-disable-next-line @typescript-eslint/no-unsafe-call,@typescript-eslint/no-unsafe-member-access
  _secp256k.secp256k1.recoverPubKey((0, _hasher.secp256k1Hasher)(hashType, message), {
    r: u8a.slice(0, 32),
    s: u8a.slice(32, 64)
  }, u8a[64]).encodeCompressed());
  const signingAddress = (0, _hasher.secp256k1Hasher)(hashType, isEthereum ? (0, _expand.secp256k1Expand)(publicKey) : publicKey);
  const inputAddress = (0, _util.u8aToU8a)(address); // for Ethereum (keccak) the last 20 bytes is the address

  return isEthereum ? (0, _util.u8aEq)(signingAddress.slice(-20), inputAddress.slice(-20)) : (0, _util.u8aEq)(signingAddress, inputAddress);
}