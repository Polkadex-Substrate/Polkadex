// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
import nacl from 'tweetnacl';
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

export function naclBoxKeypairFromSecret(secret) {
  return nacl.box.keyPair.fromSecretKey(secret.slice(0, 32));
}