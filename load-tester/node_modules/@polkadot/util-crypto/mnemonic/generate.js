"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.mnemonicGenerate = mnemonicGenerate;

var _wasmCrypto = require("@polkadot/wasm-crypto");

var _bip = require("./bip39");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
// mapping of words to the actual strength (as expected)
const STRENGTH_MAP = {
  12: 16 * 8,
  15: 20 * 8,
  18: 24 * 8,
  21: 28 * 8,
  24: 32 * 8
};
/**
 * @name mnemonicGenerate
 * @summary Creates a valid mnemonic string using using [BIP39](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki).
 * @example
 * <BR>
 *
 * ```javascript
 * import { mnemonicGenerate } from '@polkadot/util-crypto';
 *
 * const mnemonic = mnemonicGenerate(); // => string
 * ```
 */

function mnemonicGenerate(numWords = 12, onlyJs = false) {
  return (0, _wasmCrypto.isReady)() && !onlyJs ? (0, _wasmCrypto.bip39Generate)(numWords) : (0, _bip.generateMnemonic)(STRENGTH_MAP[numWords]);
}