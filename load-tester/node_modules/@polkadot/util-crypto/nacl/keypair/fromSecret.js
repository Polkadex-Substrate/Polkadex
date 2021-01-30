"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclKeypairFromSecret = naclKeypairFromSecret;

var _tweetnacl = _interopRequireDefault(require("tweetnacl"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name naclKeypairFromSecret
 * @summary Creates a new public/secret keypair from a secret.
 * @description
 * Returns a object containing a `publicKey` & `secretKey` generated from the supplied secret.
 * @example
 * <BR>
 *
 * ```javascript
 * import { naclKeypairFromSecret } from '@polkadot/util-crypto';
 *
 * naclKeypairFromSecret(...); // => { secretKey: [...], publicKey: [...] }
 * ```
 */
function naclKeypairFromSecret(secret) {
  return _tweetnacl.default.sign.keyPair.fromSecretKey(secret);
}