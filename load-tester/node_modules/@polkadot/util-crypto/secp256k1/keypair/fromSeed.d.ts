import type { Keypair } from '../../types';
/**
 * @name secp256k1KeypairFromSeed
 * @description Returns a object containing a `publicKey` & `secretKey` generated from the supplied seed.
 */
export declare function secp256k1KeypairFromSeed(seed: Uint8Array): Keypair;
