"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.idAndIndex = idAndIndex;

var _util = require("@polkadot/util");

var _utilCrypto = require("@polkadot/util-crypto");

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function retrieve(api, address) {
  try {
    // yes, this can fail, don't care too much, catch will catch it
    const decoded = (0, _util.isU8a)(address) ? address : (0, _utilCrypto.decodeAddress)((address || '').toString());

    if (decoded.length > 8) {
      const accountId = api.registry.createType('AccountId', decoded);
      return api.derive.accounts.idToIndex(accountId).pipe((0, _operators.map)(accountIndex => [accountId, accountIndex]));
    }

    const accountIndex = api.registry.createType('AccountIndex', decoded);
    return api.derive.accounts.indexToId(accountIndex.toString()).pipe((0, _operators.map)(accountId => [accountId, accountIndex]));
  } catch (error) {
    return (0, _xRxjs.of)([undefined, undefined]);
  }
}
/**
 * @name idAndIndex
 * @param {(Address | AccountId | AccountIndex | Uint8Array | string | null)} address - An accounts address in various formats.
 * @description  An array containing the [[AccountId]] and [[AccountIndex]] as optional values.
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.accounts.idAndIndex('F7Hs', ([id, ix]) => {
 *   console.log(`AccountId #${id} with corresponding AccountIndex ${ix}`);
 * });
 * ```
 */


function idAndIndex(instanceId, api) {
  return (0, _util2.memo)(instanceId, address => retrieve(api, address));
}