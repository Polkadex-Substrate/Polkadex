"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.signatureVerify = signatureVerify;

var _util = require("@polkadot/util");

var _decode = require("../address/decode");

var _verify = require("../nacl/verify");

var _verify2 = require("../schnorrkel/verify");

var _verify3 = require("../secp256k1/verify");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const secp256k1VerifyHasher = hashType => (message, signature, publicKey) => (0, _verify3.secp256k1Verify)(message, signature, publicKey, hashType);

const VERIFIERS_ECDSA = [['ecdsa', secp256k1VerifyHasher('blake2')], ['ethereum', secp256k1VerifyHasher('keccak')]];
const VERIFIERS = [['ed25519', _verify.naclVerify], ['sr25519', _verify2.schnorrkelVerify], ...VERIFIERS_ECDSA];
const CRYPTO_TYPES = ['ed25519', 'sr25519', 'ecdsa'];

function verifyDetect(result, {
  message,
  publicKey,
  signature
}, verifiers = VERIFIERS) {
  result.isValid = verifiers.some(([crypto, verify]) => {
    try {
      if (verify(message, signature, publicKey)) {
        result.crypto = crypto;
        return true;
      }
    } catch (error) {// do nothing, result.isValid still set to false
    }

    return false;
  });
  return result;
}

function verifyMultisig(result, {
  message,
  publicKey,
  signature
}) {
  (0, _util.assert)([0, 1, 2].includes(signature[0]), `Unknown crypto type, expected signature prefix [0..2], found ${signature[0]}`);
  const type = CRYPTO_TYPES[signature[0]] || 'none';
  result.crypto = type;

  try {
    result.isValid = {
      ecdsa: () => verifyDetect(result, {
        message,
        publicKey,
        signature: signature.subarray(1)
      }, VERIFIERS_ECDSA).isValid,
      ed25519: () => (0, _verify.naclVerify)(message, signature.subarray(1), publicKey),
      none: () => {
        throw Error('no verify for `none` crypto type');
      },
      sr25519: () => (0, _verify2.schnorrkelVerify)(message, signature.subarray(1), publicKey)
    }[type]();
  } catch (error) {// ignore, result.isValid still set to false
  }

  return result;
}

function signatureVerify(message, signature, addressOrPublicKey) {
  const signatureU8a = (0, _util.u8aToU8a)(signature);
  (0, _util.assert)([64, 65, 66].includes(signatureU8a.length), `Invalid signature length, expected [64..66] bytes, found ${signatureU8a.length}`);
  const result = {
    crypto: 'none',
    isValid: false
  };
  const publicKey = (0, _decode.decodeAddress)(addressOrPublicKey);
  const input = {
    message,
    publicKey,
    signature: signatureU8a
  };
  return [0, 1, 2].includes(signatureU8a[0]) && [65, 66].includes(signatureU8a.length) ? verifyMultisig(result, input) : verifyDetect(result, input);
}