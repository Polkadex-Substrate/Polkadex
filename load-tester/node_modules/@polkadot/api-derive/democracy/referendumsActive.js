"use strict";

Object.defineProperty(exports, "__esModule", {
  value: true
});
exports.referendumsActive = referendumsActive;

var _xRxjs = require("@polkadot/x-rxjs");

var _operators = require("@polkadot/x-rxjs/operators");

var _util = require("../util");

// Copyright 2017-2021 @polkadot/api-derive authors & contributors
// SPDX-License-Identifier: Apache-2.0
function referendumsActive(instanceId, api) {
  return (0, _util.memo)(instanceId, () => api.derive.democracy.referendumIds().pipe((0, _operators.switchMap)(ids => ids.length ? api.derive.democracy.referendumsInfo(ids) : (0, _xRxjs.of)([]))));
}