"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
Object.defineProperty(exports, "Keyring", {
  enumerable: true,
  get: function () {
    return _keyring.Keyring;
  }
});
Object.defineProperty(exports, "decodeAddress", {
  enumerable: true,
  get: function () {
    return _utilCrypto.decodeAddress;
  }
});
Object.defineProperty(exports, "encodeAddress", {
  enumerable: true,
  get: function () {
    return _utilCrypto.encodeAddress;
  }
});
Object.defineProperty(exports, "setSS58Format", {
  enumerable: true,
  get: function () {
    return _utilCrypto.setSS58Format;
  }
});
exports.default = void 0;

require("./detectPackage");

var _keyring = require("./keyring");

var _utilCrypto = require("@polkadot/util-crypto");

// Copyright 2017-2021 @polkadot/keyring authors & contributors
// SPDX-License-Identifier: Apache-2.0
var _default = _keyring.Keyring;
exports.default = _default;