import type { Keypair } from '../../types';
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
export declare function naclKeypairFromString(value: string): Keypair;
