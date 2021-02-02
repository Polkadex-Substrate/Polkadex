"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.pairToJson = pairToJson;

var _utilCrypto = require("@polkadot/util-crypto");

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/keyring authors & contributors
// SPDX-License-Identifier: Apache-2.0
// version 2 - nonce, encoded (previous)
// version 3 - salt, nonce, encoded
const VERSION = '3';
const ENC_NONE = ['none'];

function pairToJson(type, {
  address,
  meta
}, encoded, isEncrypted) {
  return {
    address,
    encoded: (0, _utilCrypto.base64Encode)(encoded),
    encoding: {
      content: ['pkcs8', type],
      type: isEncrypted ? _defaults.ENCODING : ENC_NONE,
      version: VERSION
    },
    meta
  };
}