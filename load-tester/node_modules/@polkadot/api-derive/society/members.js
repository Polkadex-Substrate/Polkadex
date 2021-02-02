"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.members = members;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * @description Get the member info for a society
 */
function members(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.query.society.members().pipe((0, _operators.switchMap)(members => (0, _xRxjs.combineLatest)(members.map(accountId => api.derive.society.member(accountId))))));
}