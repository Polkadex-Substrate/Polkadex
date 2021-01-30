"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.secp256k1Recover = secp256k1Recover;

var _secp256k = require("./secp256k1");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name secp256k1Recover
 * @description Recovers a publicKey from the supplied signature
 */
function secp256k1Recover(message, signature, recovery) {
  return new Uint8Array( // eslint-disable-next-line @typescript-eslint/no-unsafe-call,@typescript-eslint/no-unsafe-member-access
  _secp256k.secp256k1.recoverPubKey(message, {
    r: signature.slice(0, 32),
    s: signature.slice(32, 64)
  }, recovery).encode(null, true));
}