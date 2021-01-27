"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.decodeAddress = decodeAddress;

var _util = require("@polkadot/util");

var _decode = require("../base58/decode");

var _checksum = require("./checksum");

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
// Original implementation: https://github.com/paritytech/polka-ui/blob/4858c094684769080f5811f32b081dd7780b0880/src/polkadot.js#L6
// eslint-disable-next-line @typescript-eslint/no-unused-vars
function decodeAddress(encoded, ignoreChecksum, ss58Format = -1) {
  if ((0, _util.isU8a)(encoded) || (0, _util.isHex)(encoded)) {
    return (0, _util.u8aToU8a)(encoded);
  }

  const wrapError = message => `Decoding ${encoded}: ${message}`;

  let decoded;

  try {
    decoded = (0, _decode.base58Decode)(encoded);
  } catch (error) {
    throw new Error(wrapError(error.message));
  } // assert(defaults.allowedPrefix.includes(decoded[0] as Prefix), error('Invalid decoded address prefix'));


  (0, _util.assert)(_defaults.defaults.allowedEncodedLengths.includes(decoded.length), wrapError('Invalid decoded address length')); // TODO Unless it is an "use everywhere" prefix, throw an error
  // if (ss58Format !== -1 && (decoded[0] !== ss58Format)) {
  //   console.log(`WARN: Expected ${ss58Format}, found ${decoded[0]}`);
  // }

  const [isValid, endPos] = (0, _checksum.checkAddressChecksum)(decoded);
  (0, _util.assert)(ignoreChecksum || isValid, wrapError('Invalid decoded address checksum'));
  return decoded.slice(1, endPos);
}