"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bestNumberLag = bestNumberLag;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name bestNumberLag
 * @returns A number of blocks
 * @description Calculates the lag between finalized head and best head
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.chain.bestNumberLag((lag) => {
 *   console.log(`finalized is ${lag} blocks behind head`);
 * });
 * ```
 */
function bestNumberLag(instanceId, api) {
  return (0, _util.memo)(instanceId, () => (0, _xRxjs.combineLatest)([api.derive.chain.bestNumber(), api.derive.chain.bestNumberFinalized()]).pipe((0, _operators.map)(([bestNumber, bestNumberFinalized]) => api.registry.createType('BlockNumber', bestNumber.sub(bestNumberFinalized)))));
}