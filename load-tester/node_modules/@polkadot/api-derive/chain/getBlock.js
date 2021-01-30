"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.getBlock = getBlock;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _type = require("../type");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name getBlock
 * @param {( Uint8Array | string )} hash - A block hash as U8 array or string.
 * @description Get a specific block (e.g. rpc.chain.getBlock) and extend it with the author
 * @example
 * <BR>
 *
 * ```javascript
 * const { author, block } = await api.derive.chain.getBlock('0x123...456');
 *
 * console.log(`block #${block.header.number} was authored by ${author}`);
 * ```
 */
function getBlock(instanceId, api) {
  return (0, _util.memo)(instanceId, hash => (0, _xRxjs.combineLatest)([api.rpc.chain.getBlock(hash), api.query.system.events.at(hash), api.query.session ? api.query.session.validators.at(hash) : (0, _xRxjs.of)([])]).pipe((0, _operators.map)(([signedBlock, events, validators]) => new _type.SignedBlockExtended(api.registry, signedBlock, events, validators)), (0, _operators.catchError)(() => // where rpc.chain.getHeader throws, we will land here - it can happen that
  // we supplied an invalid hash. (Due to defaults, storage will have an
  // empty value, so only the RPC is affected). So return undefined
  (0, _xRxjs.of)())));
}