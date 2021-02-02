"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.setSS58Format = setSS58Format;

var _defaults = require("./defaults");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @description Sets the global SS58 format to use for address encoding
 * @deprecated Use keyring.setSS58Format
 */
function setSS58Format(prefix) {
  _defaults.defaults.prefix = prefix;
}