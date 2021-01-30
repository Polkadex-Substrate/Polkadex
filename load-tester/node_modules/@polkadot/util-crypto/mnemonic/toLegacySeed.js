"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.mnemonicToLegacySeed = mnemonicToLegacySeed;

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _bip = require("./bip39");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name toSeed
 * @summary Creates a valid Ethereum/Bitcoin-compatible seed from a mnemonic input
 * @example
 * <BR>
 *
 * ```javascript
 * import { mnemonicGenerate, mnemonicToBip39, mnemonicValidate } from '@polkadot/util-crypto';
 *
 * const mnemonic = mnemonicGenerate(); // => string
 * const isValidMnemonic = mnemonicValidate(mnemonic); // => boolean
 *
 * if (isValidMnemonic) {
 *   console.log(`Seed generated from mnemonic: ${mnemonicToBip39(mnemonic)}`); => u8a
 * }
 * ```
 */
function mnemonicToLegacySeed(mnemonic, password = '', onlyJs = false) {
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.bip39ToSeed)(mnemonic, password) : (0, _bip.mnemonicToSeedSync)(mnemonic, password).subarray(0, 32);
}