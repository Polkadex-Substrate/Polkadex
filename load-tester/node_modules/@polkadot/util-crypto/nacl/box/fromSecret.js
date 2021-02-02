"use strict";

var _interopRequireDefault = require("@babel/runtime/helpers/interopRequireDefault");

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.naclBoxKeypairFromSecret = naclBoxKeypairFromSecret;

var _tweetnacl = _interopRequireDefault(require("tweetnacl"));

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name naclBoxKeypairFromSecret
 * @summary Creates a new public/secret box keypair from a secret.
 * @description
 * Returns a object containing a box `publicKey` & `secretKey` generated from the supplied secret.
 * @example
 * <BR>
 *
 * ```javascript
 * import { naclBoxKeypairFromSecret } from '@polkadot/util-crypto';
 *
 * naclBoxKeypairFromSecret(...); // => { secretKey: [...], publicKey: [...] }
 * ```
 */
function naclBoxKeypairFromSecret(secret) {
  return _tweetnacl.default.box.keyPair.fromSecretKey(secret.slice(0, 32));
}