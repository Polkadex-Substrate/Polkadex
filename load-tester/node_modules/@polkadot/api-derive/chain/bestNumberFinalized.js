"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.bestNumberFinalized = bestNumberFinalized;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name bestNumberFinalized
 * @returns A BlockNumber
 * @description Get the latest finalized block number.
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.chain.bestNumberFinalized((blockNumber) => {
 *   console.log(`the current finalized block is #${blockNumber}`);
 * });
 * ```
 */
function bestNumberFinalized(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.rpc.chain.subscribeFinalizedHeads().pipe((0, _operators.map)(header => header.number.unwrap())));
}