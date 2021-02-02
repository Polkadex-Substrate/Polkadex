"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.sqrtElectorate = sqrtElectorate;

var _util = require("@polkadot/util");

var _operators = require("@polkadot/x-rxjs/operators");

var _util2 = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function sqrtElectorate(instanceId, api) {
  return (0, _util2.memo)(instanceId, () => api.query.balances.totalIssuance().pipe((0, _operators.map)(totalIssuance => (0, _util.bnSqrt)(totalIssuance))));
}