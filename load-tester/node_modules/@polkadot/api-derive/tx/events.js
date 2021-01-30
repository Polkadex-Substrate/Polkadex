"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.events = events;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function events(instanceId, api) {
  return (0, _util.memo)(instanceId, at => (0, _xRxjs.combineLatest)([api.query.system.events.at(at), api.rpc.chain.getBlock(at)]).pipe((0, _operators.map)(([events, block]) => ({
    block,
    events
  }))));
}