"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.info = info;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @description Get the overall info for a society
 */
function info(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.queryMulti([api.query.society.bids, api.query.society.defender, api.query.society.founder, api.query.society.head, api.query.society.maxMembers, api.query.society.pot]).pipe((0, _operators.map)(([bids, defender, founder, head, maxMembers, pot]) => ({
    bids,
    defender: defender.unwrapOr(undefined),
    founder: founder.unwrapOr(undefined),
    hasDefender: defender.isSome && head.isSome && !head.eq(defender) || false,
    head: head.unwrapOr(undefined),
    maxMembers,
    pot
  }))));
}