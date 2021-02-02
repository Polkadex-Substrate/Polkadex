"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bestNumber = bestNumber;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name bestNumber
 * @returns The latest block number.
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.chain.bestNumber((blockNumber) => {
 *   console.log(`the current best block is #${blockNumber}`);
 * });
 * ```
 */
function bestNumber(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.derive.chain.subscribeNewHeads().pipe((0, _operators.map)(header => header.number.unwrap())));
}