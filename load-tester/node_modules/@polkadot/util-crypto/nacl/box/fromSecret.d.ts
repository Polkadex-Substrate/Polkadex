import type { Keypair } from '../../types';
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
export declare function naclBoxKeypairFromSecret(secret: Uint8Array): Keypair;
