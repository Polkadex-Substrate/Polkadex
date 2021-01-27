"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decodePair = decodePair;

var _util = require("@polkadot/util");

var _utilCrypto = require("@polkadot/util-crypto");

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/keyring authors & contributors
// SPDX-License-Identifier: Apache-2.0
const SEED_OFFSET = _defaults.PKCS8_HEADER.length;

function decodePkcs8(encoded) {
  const header = encoded.subarray(0, _defaults.PKCS8_HEADER.length);
  (0, _util.assert)(header.toString() === _defaults.PKCS8_HEADER.toString(), 'Invalid Pkcs8 header found in body');
  let secretKey = encoded.subarray(SEED_OFFSET, SEED_OFFSET + _defaults.SEC_LENGTH);
  let divOffset = SEED_OFFSET + _defaults.SEC_LENGTH;
  let divider = encoded.subarray(divOffset, divOffset + _defaults.PKCS8_DIVIDER.length); // old-style, we have the seed here

  if (divider.toString() !== _defaults.PKCS8_DIVIDER.toString()) {
    divOffset = SEED_OFFSET + _defaults.SEED_LENGTH;
    secretKey = encoded.subarray(SEED_OFFSET, divOffset);
    divider = encoded.subarray(divOffset, divOffset + _defaults.PKCS8_DIVIDER.length);
  }

  (0, _util.assert)(divider.toString() === _defaults.PKCS8_DIVIDER.toString(), 'Invalid Pkcs8 divider found in body');
  const pubOffset = divOffset + _defaults.PKCS8_DIVIDER.length;
  const publicKey = encoded.subarray(pubOffset, pubOffset + _defaults.PUB_LENGTH);
  return {
    publicKey,
    secretKey
  };
}

function decodePair(passphrase, encrypted, encType = _defaults.ENCODING) {
  (0, _util.assert)(encrypted, 'No encrypted data available to decode');
  (0, _util.assert)(passphrase || !encType.includes('xsalsa20-poly1305'), 'Password required to decode encypted data');
  let encoded = encrypted;

  if (passphrase) {
    let password;

    if (encType.includes('scrypt')) {
      const {
        params,
        salt
      } = (0, _utilCrypto.scryptFromU8a)(encrypted);
      password = (0, _utilCrypto.scryptEncode)(passphrase, salt, params).password;
      encrypted = encrypted.subarray(_defaults.SCRYPT_LENGTH);
    } else {
      password = (0, _util.stringToU8a)(passphrase);
    }

    encoded = (0, _utilCrypto.naclDecrypt)(encrypted.subarray(_defaults.NONCE_LENGTH), encrypted.subarray(0, _defaults.NONCE_LENGTH), (0, _util.u8aFixLength)(password, 256, true));
  }

  (0, _util.assert)(encoded, 'Unable to decode using the supplied passphrase');
  return decodePkcs8(encoded);
}