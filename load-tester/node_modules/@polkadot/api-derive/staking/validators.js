"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.nextElected = nextElected;
exports.validators = validators;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function nextElected(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.query.staking.erasStakers ? api.derive.session.indexes().pipe( // only populate for next era in the last session, so track both here - entries are not
  // subscriptions, so we need a trigger - currentIndex acts as that trigger to refresh
  (0, _operators.switchMap)(({
    currentEra
  }) => api.query.staking.erasStakers.keys(currentEra)), (0, _operators.map)(keys => keys.map(key => key.args[1]))) : api.query.staking.currentElected());
}
/**
 * @description Retrieve latest list of validators
 */


function validators(instanceId, api) {
  return (0, _util.memo)(instanceId, () => // Sadly the node-template is (for some obscure reason) not comprehensive, so while the derive works
  // in all actual real-world deployed chains, it does create some confusion for limited template chains
  (0, _xRxjs.combineLatest)([api.query.session ? api.query.session.validators() : (0, _xRxjs.of)([]), api.query.staking ? api.derive.staking.nextElected() : (0, _xRxjs.of)([])]).pipe((0, _operators.map)(([validators, nextElected]) => ({
    nextElected: nextElected.length ? nextElected : validators,
    validators
  }))));
}