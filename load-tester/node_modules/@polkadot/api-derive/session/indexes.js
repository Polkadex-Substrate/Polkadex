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
// parse into Indexes
function parse([activeEra, activeEraStart, currentEra, currentIndex, validatorCount]) {
  return {
    activeEra,
    activeEraStart,
    currentEra,
    currentIndex,
    validatorCount
  };
} // query based on latest


function query(api) {
  return api.queryMulti([api.query.staking.activeEra, api.query.staking.currentEra, api.query.session.currentIndex, api.query.staking.validatorCount]).pipe((0, _operators.map)(([activeOpt, currentEra, currentIndex, validatorCount]) => {
    const {
      index,
      start
    } = activeOpt.unwrapOrDefault();
    return parse([index, start, currentEra.unwrapOrDefault(), currentIndex, validatorCount]);
  }));
} // empty set when none is available


function empty(api) {
  return (0, _xRxjs.of)(parse([api.registry.createType('EraIndex'), api.registry.createType('Option<Moment>'), api.registry.createType('EraIndex'), api.registry.createType('SessionIndex', 1), api.registry.createType('u32')]));
}

function indexes(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.query.session && api.query.staking ? query(api) : empty(api));
}