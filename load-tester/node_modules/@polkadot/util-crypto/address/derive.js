"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.deriveAddress = deriveAddress;

var _util = require("@polkadot/util");

var _key = require("../key");

var _schnorrkel = require("../schnorrkel");

var _decode = require("./decode");

var _encode = require("./encode");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name deriveAddress
 * @summary Creates a sr25519 derived address from the supplied and path.
 * @description
 * Creates a sr25519 derived address based on the input address/publicKey and the uri supplied.
 */
function deriveAddress(who, suri, ss58Format) {
  const {
    path
  } = (0, _key.keyExtractPath)(suri);
  (0, _util.assert)(path.length && !path.some(path => path.isHard), 'Expected suri to contain a combination of non-hard paths');
  return (0, _encode.encodeAddress)(path.reduce((publicKey, path) => {
    return (0, _schnorrkel.schnorrkelDerivePublic)(publicKey, path.chainCode);
  }, (0, _decode.decodeAddress)(who)), ss58Format);
}