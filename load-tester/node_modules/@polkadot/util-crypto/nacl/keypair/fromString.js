"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclKeypairFromString = naclKeypairFromString;

var _util = require("@polkadot/util");

var _asU8a = require("../../blake2/asU8a");

var _fromSeed = require("./fromSeed");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name naclKeypairFromString
 * @summary Creates a new public/secret keypair from a string.
 * @description
 * Returns a object containing a `publicKey` & `secretKey` generated from the supplied string. The string is hashed and the value used as the input seed.
 * @example
 * <BR>
 *
 * ```javascript
 * import { naclKeypairFromString } from '@polkadot/util-crypto';
 *
 * naclKeypairFromString('test'); // => { secretKey: [...], publicKey: [...] }
 * ```
 */
function naclKeypairFromString(value) {
  return (0, _fromSeed.naclKeypairFromSeed)((0, _asU8a.blake2AsU8a)((0, _util.stringToU8a)(value), 256));
}