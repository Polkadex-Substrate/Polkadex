"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.eraLength = eraLength;

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function eraLength(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.derive.session.info().pipe((0, _operators.map)(info => info.eraLength)));
}