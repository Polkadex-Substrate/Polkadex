"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.candidates = candidates;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @description Get the candidate info for a society
 */
function candidates(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.query.society.candidates().pipe((0, _operators.switchMap)(candidates => (0, _xRxjs.combineLatest)([(0, _xRxjs.of)(candidates), api.query.society.suspendedCandidates.multi(candidates.map(({
    who
  }) => who))])), (0, _operators.map)(([candidates, suspended]) => candidates.map(({
    kind,
    value,
    who
  }, index) => ({
    accountId: who,
    isSuspended: suspended[index].isSome,
    kind,
    value
  })))));
}