"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.indexToId = indexToId;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name indexToId
 * @param {( AccountIndex | string )} accountIndex - An accounts index in different formats.
 * @returns Returns the corresponding AccountId.
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.accounts.indexToId('F7Hs', (accountId) => {
 *   console.log(`The AccountId of F7Hs is ${accountId}`);
 * });
 * ```
 */
function indexToId(instanceId, api) {
  return (0, _util.memo)(instanceId, accountIndex => api.query.indices ? api.query.indices.accounts(accountIndex).pipe((0, _operators.map)(optResult => optResult.unwrapOr([])[0])) : (0, _xRxjs.of)(undefined));
}