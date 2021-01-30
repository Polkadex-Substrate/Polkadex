"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclKeypairFromRandom = naclKeypairFromRandom;

var _tweetnacl = _interopRequireDefault(require("tweetnacl"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name naclKeypairFromRandom
 * @summary Creates a new public/secret keypair.
 * @description
 * Returns a new generate object containing a `publicKey` & `secretKey`.
 * @example
 * <BR>
 *
 * ```javascript
 * import { naclKeypairFromRandom } from '@polkadot/util-crypto';
 *
 * naclKeypairFromRandom(); // => { secretKey: [...], publicKey: [...] }
 * ```
 */
function naclKeypairFromRandom() {
  return _tweetnacl.default.sign.keyPair();
}