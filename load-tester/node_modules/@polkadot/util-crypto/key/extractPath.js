"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.keyExtractPath = keyExtractPath;

var _util = require("@polkadot/util");

var _DeriveJunction = require("./DeriveJunction");

// Copyright 2017-2021 @polkadot/util-crypto authors & contributors
// SPDX-License-Identifier: Apache-2.0
const RE_JUNCTION = /\/(\/?)([^/]+)/g;

/**
 * @description Extract derivation junctions from the supplied path
 */
function keyExtractPath(derivePath) {
  const parts = derivePath.match(RE_JUNCTION);
  const path = [];
  let constructed = '';

  if (parts) {
    constructed = parts.join('');
    parts.forEach(value => {
      path.push(_DeriveJunction.DeriveJunction.from(value.substr(1)));
    });
  }

  (0, _util.assert)(constructed === derivePath, `Re-constructed path "${constructed}" does not match input`);
  return {
    parts,
    path
  };
}