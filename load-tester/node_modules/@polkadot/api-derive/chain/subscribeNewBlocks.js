"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.subscribeNewBlocks = subscribeNewBlocks;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _type = require("../type");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name subscribeNewBlocks
 * @returns The latest block & events for that block
 */
function subscribeNewBlocks(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.derive.chain.subscribeNewHeads().pipe((0, _operators.switchMap)(header => {
    const blockHash = header.hash;
    return (0, _xRxjs.combineLatest)(api.rpc.chain.getBlock(blockHash), api.query.system.events.at(blockHash), (0, _xRxjs.of)(header.validators));
  }), (0, _operators.map)(([block, events, validators]) => new _type.SignedBlockExtended(api.registry, block, events, validators))));
}