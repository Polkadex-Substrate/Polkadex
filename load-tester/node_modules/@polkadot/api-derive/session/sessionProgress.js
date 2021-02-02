"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.sessionProgress = sessionProgress;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function sessionProgress(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.derive.session.progress().pipe((0, _operators.map)(info => info.sessionProgress)));
}