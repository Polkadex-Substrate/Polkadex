"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.indexes = indexes;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
let indicesCache = null;

function queryAccounts(api) {
  return api.query.indices.accounts.entries().pipe((0, _operators.map)(entries => entries.reduce((indexes, [key, idOpt]) => {
    if (idOpt.isSome) {
      indexes[idOpt.unwrap()[0].toString()] = key.args[0];
    }

    return indexes;
  }, {})));
}
/**
 * @name indexes
 * @returns Returns all the indexes on the system.
 * @description This is an unwieldly query since it loops through
 * all of the enumsets and returns all of the values found. This could be up to 32k depending
 * on the number of active accounts in the system
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.accounts.indexes((indexes) => {
 *   console.log('All existing AccountIndexes', indexes);
 * });
 * ```
 */


function indexes(instanceId, api) {
  return (0, _util.memo)(instanceId, () => indicesCache ? (0, _xRxjs.of)(indicesCache) : (api.query.indices ? queryAccounts(api).pipe((0, _operators.startWith)({})) : (0, _xRxjs.of)({})).pipe((0, _operators.map)(indices => {
    indicesCache = indices;
    return indices;
  })));
}