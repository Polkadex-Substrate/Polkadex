"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.votingBalances = votingBalances;

var _xRxjs = require("@polkadot/x-rxjs");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function votingBalances(instanceId, api) {
  return (0, _util.memo)(instanceId, addresses => !addresses || !addresses.length ? (0, _xRxjs.of)([]) : (0, _xRxjs.combineLatest)(addresses.map(accountId => api.derive.balances.account(accountId))));
}