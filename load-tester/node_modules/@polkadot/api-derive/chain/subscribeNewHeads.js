"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.subscribeNewHeads = subscribeNewHeads;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _type = require("../type");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @name subscribeNewHeads
 * @returns A header with the current header (including extracted author)
 * @description An observable of the current block header and it's author
 * @example
 * <BR>
 *
 * ```javascript
 * api.derive.chain.subscribeNewHeads((header) => {
 *   console.log(`block #${header.number} was authored by ${header.author}`);
 * });
 * ```
 */
function subscribeNewHeads(instanceId, api) {
  return (0, _util.memo)(instanceId, () => (0, _xRxjs.combineLatest)([api.rpc.chain.subscribeNewHeads(), api.query.session ? api.query.session.validators() : (0, _xRxjs.of)([])]).pipe((0, _operators.map)(([header, validators]) => new _type.HeaderExtended(api.registry, header, validators))));
}