"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.accountId = accountId;

var _util = require("@polkadot/util");

var _utilCrypto = require("@polkadot/util-crypto");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function retrieve(api, address) {
  const decoded = (0, _util.isU8a)(address) ? address : (0, _utilCrypto.decodeAddress)((address || '').toString());

  if (decoded.length > 8) {
    return (0, _xRxjs.of)(api.registry.createType('AccountId', decoded));
  }

  const accountIndex = api.registry.createType('AccountIndex', decoded);
  return api.derive.accounts.indexToId(accountIndex.toString()).pipe((0, _operators.map)(accountId => (0, _util.assertReturn)(accountId, 'Unable to retrieve accountId')));
}
/**
 * @name accountId
 * @param {(Address | AccountId | AccountIndex | string | null)} address - An accounts address in various formats.
 * @description  An [[AccountId]]
 */


function accountId(instanceId, api) {
  return (0, _util2.memo)(instanceId, address => retrieve(api, address));
}